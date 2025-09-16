mod models;
mod scraper;
mod schema;
mod actor;
mod dataset;
mod utils;

use anyhow::Result;
use scraper::ShopifyScraper;
use schema::{ScraperInput, OutputFormat};
use actor::Actor;
use tracing::{info, error, warn};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with proper formatting for Apify
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();
    
    info!("🚀 Starting Shopify Lightning Scraper");
    
    // Initialize Actor for Apify integration
    let mut actor = Actor::new();
    
    // Load input from Apify or use default
    let input = load_input().await.map_err(|e| {
        error!("Failed to load input: {}", e);
        e
    })?;
    
    info!("Scraper Configuration:");
    info!("Domain: {}", input.domain);
    info!("Max Products: {}", input.max_products);
    info!("Concurrent Requests: {}", input.max_concurrent);
    info!("Auto Discover: {}", input.auto_discover);
    
    // Create scraper
    let scraper = ShopifyScraper::new(input.clone())?;
    
    // Determine product handles
    let product_handles = if !input.product_handles.is_empty() {
        info!("Using provided product handles: {}", input.product_handles.len());
        input.product_handles
    } else if input.auto_discover {
        info!("Auto-discovering products...");
        let discovered = scraper.discover_products(&input.domain, input.max_products).await?;
        if discovered.is_empty() {
            warn!("No products discovered. Exiting.");
            return Ok(());
        }
        info!("Discovered {} product handles", discovered.len());
        discovered
    } else {
        error!("Error: Must specify either product_handles or auto_discover");
        return Ok(());
    };
    
    // Scrape products
    info!("Starting to scrape {} products", product_handles.len());
    let products = scraper.scrape_multiple_products(&input.domain, product_handles).await?;
    
    if !products.is_empty() {
        info!("Successfully scraped {} products", products.len());
        
        // Convert products to JSON values for Apify storage
        let json_products: Vec<Value> = products.iter()
            .map(|p| serde_json::to_value(p).unwrap())
            .collect();
        
        // Save to Apify dataset
        match actor.push_data(&json_products).await {
            Ok(_) => {
                info!("Successfully saved {} products to Apify dataset", json_products.len());
            }
            Err(e) => {
                error!("Failed to save data to Apify dataset: {}", e);
                // Fallback: print to stdout
                for product in &products {
                    let json_output = serde_json::to_string_pretty(product)?;
                    println!("{}", json_output);
                }
            }
        }
        
        // Also output in requested format for debugging
        match input.output_format {
            OutputFormat::Json => {
                let json_output = serde_json::to_string_pretty(&products)?;
                info!("JSON Output:\n{}", json_output);
            }
            OutputFormat::JsonL => {
                for product in &products {
                    let json_line = serde_json::to_string(product)?;
                    info!("JSONL: {}", json_line);
                }
            }
            OutputFormat::Csv => {
                let mut wtr = csv::Writer::from_writer(std::io::stdout());
                for product in &products {
                    wtr.serialize(product)?;
                }
                wtr.flush()?;
            }
            OutputFormat::Xml => {
                warn!("XML output not implemented yet");
            }
            OutputFormat::Parquet => {
                warn!("Parquet output not implemented yet");
            }
        }
    } else {
        warn!("No products found.");
    }
    
    info!("Scraping completed successfully");
    Ok(())
}

async fn load_input() -> Result<ScraperInput> {
    // Try to load from Apify input first
    if utils::is_on_apify() {
        match load_apify_input().await {
            Ok(input) => {
                info!("Loaded input from Apify");
                return Ok(input);
            }
            Err(e) => {
                warn!("Failed to load Apify input: {}", e);
            }
        }
    }
    
    // Fallback to default input
    info!("Using default input configuration");
    Ok(ScraperInput {
        domain: "samapura.store".to_string(),
        product_handles: vec![],
        auto_discover: true,
        max_products: 50,
        max_concurrent: 100,
        timeout_seconds: 30,
        output_format: OutputFormat::Json,
        filters: schema::ProductFilters::default(),
        extraction: schema::ExtractionOptions::default(),
        performance: schema::PerformanceSettings {
            enable_http2: false,
            ..Default::default()
        },
    })
}

async fn load_apify_input() -> Result<ScraperInput> {
    use std::fs;
    
    // Read INPUT.json from Apify's local storage
    let json_text = fs::read_to_string("INPUT.json")?;
    
    // Parse the JSON input
    let input_value: Value = serde_json::from_str(&json_text)?;
    
    // Convert to ScraperInput
    let input = ScraperInput {
        domain: input_value.get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("samapura.store")
            .to_string(),
        product_handles: input_value.get("product_handles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        auto_discover: input_value.get("auto_discover")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        max_products: input_value.get("max_products")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize,
        max_concurrent: input_value.get("max_concurrent")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize,
        timeout_seconds: input_value.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as u64,
        output_format: input_value.get("output_format")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "json" => Some(OutputFormat::Json),
                "jsonl" => Some(OutputFormat::JsonL),
                "csv" => Some(OutputFormat::Csv),
                "xml" => Some(OutputFormat::Xml),
                "parquet" => Some(OutputFormat::Parquet),
                _ => None,
            })
            .unwrap_or(OutputFormat::Json),
        filters: schema::ProductFilters::default(),
        extraction: schema::ExtractionOptions::default(),
        performance: schema::PerformanceSettings {
            enable_http2: false,
            ..Default::default()
        },
    };
    
    Ok(input)
}