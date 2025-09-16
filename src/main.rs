mod models;
mod scraper;
mod schema;
mod actor;
mod dataset;
mod utils;
mod multi_website_scraper;
mod data_wrangling;
mod lightning_scraper;

use anyhow::Result;
use scraper::ShopifyScraper;
use multi_website_scraper::{MultiWebsiteScraper, MultiWebsiteConfig};
use lightning_scraper::{LightningScraper, LightningConfig};
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
    info!("Multi-website mode: {}", input.multi_website_mode);
    
    if input.multi_website_mode {
        info!("Websites to scrape: {}", input.websites.len());
        for website in &input.websites {
            info!("  - {}", website);
        }
    } else {
        info!("Domain: {}", input.domain);
    }
    
    info!("Max Products per site: {}", input.max_products);
    info!("Concurrent Requests per domain: {}", input.max_concurrent);
    info!("Global Concurrent Requests: {}", input.global_max_concurrent);
    info!("Auto Discover: {}", input.auto_discover);
    
    if input.multi_website_mode {
        // Multi-website mode - choose between standard and lightning-fast
        if input.websites.is_empty() {
            error!("Error: Multi-website mode enabled but no websites specified");
            return Ok(());
        }
        
        // Check if lightning mode is enabled (via custom field or environment)
        let lightning_mode = std::env::var("LIGHTNING_MODE").unwrap_or_else(|_| "true".to_string()) == "true";
        
        if lightning_mode {
            info!("⚡ Starting LIGHTNING-FAST multi-website scraping mode");
            
            // Create lightning scraper for maximum speed
            let lightning_config = LightningConfig {
                websites: input.websites.clone(),
                max_products_per_site: input.max_products,
                max_concurrent_per_domain: input.max_concurrent,
                global_max_concurrent: input.global_max_concurrent,
                cache_ttl_seconds: input.caching.cache_ttl_seconds,
                timeout_seconds: input.timeout_seconds,
                enable_zero_copy: true,
                enable_simd: true,
                enable_memory_pool: true,
                batch_size: 1000,
            };
            
            let lightning_scraper = LightningScraper::new(lightning_config.clone(), input.clone())?;
            let results = lightning_scraper.scrape_lightning_fast(lightning_config).await?;
            
            // Process results from all websites
            let mut all_products = Vec::new();
            let mut total_products = 0;
            
            for (website, products) in &results {
                info!("⚡ Website {}: {} products", website, products.len());
                total_products += products.len();
                all_products.extend(products.clone());
            }
            
            if !all_products.is_empty() {
                info!("⚡ Successfully scraped {} total products from {} websites", 
                      total_products, results.len());
                
                // Convert products to JSON values for Apify storage
                let json_products: Vec<Value> = all_products.iter()
                    .map(|p| serde_json::to_value(p).unwrap())
                    .collect();
                
                // Save to Apify dataset
                match actor.push_data(&json_products).await {
                    Ok(_) => {
                        info!("⚡ Successfully saved {} products to Apify dataset", json_products.len());
                    }
                    Err(e) => {
                        error!("Failed to save data to Apify dataset: {}", e);
                    }
                }
            } else {
                warn!("No products found from any website.");
            }
        } else {
            info!("🚀 Starting standard multi-website scraping mode");
            
            // Create multi-website scraper
            let multi_config = MultiWebsiteConfig {
                websites: input.websites.clone(),
                max_products_per_site: input.max_products,
                max_concurrent_per_domain: input.max_concurrent,
                global_max_concurrent: input.global_max_concurrent,
                cache_ttl_seconds: input.caching.cache_ttl_seconds,
                retry_delay_ms: input.caching.retry_delay_ms,
                max_retries: input.caching.max_retries,
                enable_response_caching: input.caching.enable_response_caching,
                enable_failure_caching: input.caching.enable_failure_caching,
                rate_limit_per_domain_ms: input.caching.rate_limit_per_domain_ms,
                timeout_seconds: input.timeout_seconds,
            };
            
            let multi_scraper = MultiWebsiteScraper::new(multi_config.clone(), input.clone())?;
            let results = multi_scraper.scrape_multiple_websites(multi_config).await?;
            
            // Process results from all websites
            let mut all_products = Vec::new();
            let mut total_products = 0;
            
            for (website, products) in &results {
                info!("Website {}: {} products", website, products.len());
                total_products += products.len();
                all_products.extend(products.clone());
            }
            
            if !all_products.is_empty() {
                info!("Successfully scraped {} total products from {} websites", 
                      total_products, results.len());
                
                // Convert products to JSON values for Apify storage
                let json_products: Vec<Value> = all_products.iter()
                    .map(|p| serde_json::to_value(p).unwrap())
                    .collect();
                
                // Save to Apify dataset
                match actor.push_data(&json_products).await {
                    Ok(_) => {
                        info!("Successfully saved {} products to Apify dataset", json_products.len());
                    }
                    Err(e) => {
                        error!("Failed to save data to Apify dataset: {}", e);
                    }
                }
            } else {
                warn!("No products found from any website.");
            }
        }
        
    } else {
        // Single website mode (original logic)
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
                }
            }
        } else {
            warn!("No products found.");
        }
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
    
    // Fallback to INPUT.json file
    match load_json_input().await {
        Ok(input) => {
            info!("Loaded input from INPUT.json");
            return Ok(input);
        }
        Err(e) => {
            warn!("Failed to load INPUT.json: {}", e);
        }
    }
    
    // Final fallback to default input
    info!("Using default input configuration");
    Ok(ScraperInput {
        domain: "samapura.store".to_string(),
        websites: vec!["samapura.store".to_string()],
        multi_website_mode: false,
        product_handles: vec![],
        auto_discover: true,
        max_products: 50,
        max_concurrent: 100,
        global_max_concurrent: 200,
        timeout_seconds: 30,
        output_format: OutputFormat::Json,
        filters: schema::ProductFilters::default(),
        extraction: schema::ExtractionOptions::default(),
        performance: schema::PerformanceSettings::default(),
        caching: schema::CachingSettings::default(),
    })
}

async fn load_json_input() -> Result<ScraperInput> {
    let input_str = tokio::fs::read_to_string("INPUT.json").await?;
    let input: ScraperInput = serde_json::from_str(&input_str)?;
    Ok(input)
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
        websites: input_value.get("websites")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["samapura.store".to_string()]),
        multi_website_mode: input_value.get("multi_website_mode")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
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
        global_max_concurrent: input_value.get("global_max_concurrent")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as usize,
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
        performance: schema::PerformanceSettings::default(),
        caching: schema::CachingSettings::default(),
    };
    
    Ok(input)
}