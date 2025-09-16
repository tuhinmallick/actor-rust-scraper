mod models;
mod scraper;
mod schema;

use anyhow::Result;
use scraper::ShopifyScraper;
use schema::{ScraperInput, OutputFormat};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Example usage with samapura.store
    let input = ScraperInput {
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
    };
    
    println!("🚀 Shopify Lightning Scraper");
    println!("=============================");
    println!("Domain: {}", input.domain);
    println!("Max Products: {}", input.max_products);
    println!("Concurrent Requests: {}", input.max_concurrent);
    
    // Create scraper
    let scraper = ShopifyScraper::new(input.clone())?;
    
    // Determine product handles
    let product_handles = if !input.product_handles.is_empty() {
        input.product_handles
    } else if input.auto_discover {
        let discovered = scraper.discover_products(&input.domain, input.max_products).await?;
        if discovered.is_empty() {
            eprintln!("No products discovered. Exiting.");
            return Ok(());
        }
        println!("Discovered {} product handles", discovered.len());
        discovered
    } else {
        eprintln!("Error: Must specify either product_handles or auto_discover");
        return Ok(());
    };
    
    // Scrape products
    let products = scraper.scrape_multiple_products(&input.domain, product_handles).await?;
    
    if !products.is_empty() {
        match input.output_format {
            OutputFormat::Json => {
                let json_output = serde_json::to_string_pretty(&products)?;
                println!("{}", json_output);
            }
            OutputFormat::JsonL => {
                for product in products {
                    let json_line = serde_json::to_string(&product)?;
                    println!("{}", json_line);
                }
            }
            OutputFormat::Csv => {
                let mut wtr = csv::Writer::from_writer(std::io::stdout());
                for product in products {
                    wtr.serialize(product)?;
                }
                wtr.flush()?;
            }
            OutputFormat::Xml => {
                // XML output would be implemented here
                println!("XML output not implemented yet");
            }
            OutputFormat::Parquet => {
                // Parquet output would be implemented here
                println!("Parquet output not implemented yet");
            }
        }
    } else {
        println!("No products found.");
    }
    
    Ok(())
}