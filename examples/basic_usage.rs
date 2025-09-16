use anyhow::Result;
use shopify_scraper::{ShopifyScraper, format_output, OutputFormat};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Example domain (replace with actual Shopify store)
    let domain = "example.myshopify.com";
    
    // Example product handles (replace with actual product handles)
    let product_handles = vec![
        "awesome-t-shirt".to_string(),
        "cool-hoodie".to_string(),
        "amazing-jeans".to_string(),
    ];
    
    println!("Scraping products from {}", domain);
    println!("Product handles: {:?}", product_handles);
    
    // Use the scraper
    let scraper = ShopifyScraper::new(50, 10)?;
    
    // Scrape multiple products in parallel
    let products = scraper.scrape_multiple_products(domain, product_handles).await?;
    
    println!("\nScraped {} products:", products.len());
    
    for product in &products {
        println!("\n--- {} ---", product.title);
        println!("ID: {}", product.id);
        println!("Price: {} {}", product.currency, product.price);
        println!("Vendor: {}", product.vendor);
        println!("Available: {}", product.availability);
        println!("Variants: {}", product.variants.len());
        println!("Images: {}", product.images.len());
    }
    
    // Example of auto-discovery
    println!("\n--- Auto-Discovery Example ---");
    let discovered_handles = scraper.discover_products(domain, 10).await?;
    println!("Discovered {} product handles:", discovered_handles.len());
    for handle in discovered_handles.iter().take(5) {
        println!("  - {}", handle);
    }
    
    // Example of output formatting
    println!("\n--- Output Formatting Example ---");
    let json_output = format_output(&products, &OutputFormat::Json)?;
    println!("JSON output length: {} characters", json_output.len());
    
    let csv_output = format_output(&products, &OutputFormat::Csv)?;
    println!("CSV output length: {} characters", csv_output.len());
    
    Ok(())
}