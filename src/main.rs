mod models;
mod scraper;
mod output;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use scraper::ShopifyScraper;
use output::{format_output, OutputFormat};

/// High-Performance Shopify Product Scraper
#[derive(Parser)]
#[command(name = "shopify-scraper")]
#[command(about = "High-Performance Shopify Product Scraper")]
#[command(version)]
struct Cli {
    /// Shopify store domain (e.g., store.myshopify.com)
    domain: String,
    
    /// Specific product handles to scrape
    #[arg(short, long, value_name = "HANDLE")]
    products: Vec<String>,
    
    /// Auto-discover products from store
    #[arg(short, long)]
    discover: bool,
    
    /// Maximum products to scrape
    #[arg(short, long, default_value = "100")]
    max_products: usize,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    output: OutputFormat,
    
    /// Max concurrent requests
    #[arg(short, long, default_value = "100")]
    concurrent: usize,
    
    /// Request timeout in seconds
    #[arg(short, long, default_value = "10")]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Determine product handles
    let product_handles = if !cli.products.is_empty() {
        cli.products
    } else if cli.discover {
        let scraper = ShopifyScraper::new(cli.concurrent, cli.timeout)?;
        let discovered = scraper.discover_products(&cli.domain, cli.max_products).await?;
        if discovered.is_empty() {
            eprintln!("No products discovered. Exiting.");
            return Ok(());
        }
        discovered
    } else {
        eprintln!("Error: Must specify either --products or --discover");
        return Ok(());
    };
    
    // Scrape products
    let scraper = ShopifyScraper::new(cli.concurrent, cli.timeout)?;
    let products = scraper.scrape_multiple_products(&cli.domain, product_handles).await?;
    
    if !products.is_empty() {
        let output = format_output(&products, &cli.output)?;
        println!("{}", output);
    } else {
        println!("No products found.");
    }
    
    Ok(())
}