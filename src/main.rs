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
    
    // Example usage with purelei.com
    let input = ScraperInput {
        domain: "purelei.com".to_string(),
        product_handles: vec![],
        auto_discover: true,
        max_products: 200,
        max_concurrent: 500,
        timeout_seconds: 30,
        output_format: OutputFormat::Json,
        filters: schema::ProductFilters::default(),
        extraction: schema::ExtractionOptions::default(),
        performance: schema::PerformanceSettings {
            enable_http2: false,
            enable_compression: true,
            enable_connection_pooling: true,
            enable_retries: true,
            max_retries: 5,
            retry_delay_ms: 100,
            enable_deduplication: true,
            ..Default::default()
        },
    };
    
    println!("🚀 Shopify Lightning Scraper");
    println!("=============================");
    println!("Domain: {}", input.domain);
    println!("Max Products: {}", input.max_products);
    println!("Concurrent Requests: {}", input.max_concurrent);
    
    let total_start = std::time::Instant::now();
    
    // Create scraper
    let scraper = ShopifyScraper::new(input.clone())?;
    
    // Determine product handles
    let discovery_start = std::time::Instant::now();
    let product_handles = if !input.product_handles.is_empty() {
        input.product_handles
    } else if input.auto_discover {
        let discovered = scraper.discover_products(&input.domain, input.max_products).await?;
        if discovered.is_empty() {
            eprintln!("No products discovered. Exiting.");
            return Ok(());
        }
        let discovery_time = discovery_start.elapsed();
        println!("✅ Discovered {} product handles in {:.3}ms", discovered.len(), discovery_time.as_secs_f64() * 1000.0);
        discovered
    } else {
        eprintln!("Error: Must specify either product_handles or auto_discover");
        return Ok(());
    };
    
    // Scrape products
    let scraping_start = std::time::Instant::now();
    let products = scraper.scrape_multiple_products(&input.domain, product_handles).await?;
    let scraping_time = scraping_start.elapsed();
    
    let total_time = total_start.elapsed();
    
    if !products.is_empty() {
        println!("⚡ Performance Metrics:");
        println!("  📊 Total Products: {}", products.len());
        println!("  ⏱️  Discovery Time: {:.3}ms", discovery_start.elapsed().as_secs_f64() * 1000.0);
        println!("  🚀 Scraping Time: {:.3}ms", scraping_time.as_secs_f64() * 1000.0);
        println!("  📈 Total Time: {:.3}ms", total_time.as_secs_f64() * 1000.0);
        println!("  ⚡ Avg per Product: {:.3}ms", scraping_time.as_secs_f64() * 1000.0 / products.len() as f64);
        println!("  🔥 Products/sec: {:.1}", products.len() as f64 / scraping_time.as_secs_f64());
        println!("");
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