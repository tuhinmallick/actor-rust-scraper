use anyhow::Result;
use shopify_scraper::ShopifyScraper;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let domain = "shopify.dev"; // Using a public Shopify store for testing
    let test_handles = vec![
        "test-product".to_string(),
        "another-product".to_string(),
        "third-product".to_string(),
    ];
    
    println!("🚀 Shopify Scraper Performance Benchmark");
    println!("==========================================");
    println!("Domain: {}", domain);
    println!("Test handles: {:?}", test_handles);
    
    // Test different concurrency levels
    let concurrency_levels = vec![10, 50, 100, 200];
    
    for concurrent in concurrency_levels {
        println!("\n--- Testing with {} concurrent requests ---", concurrent);
        
        let scraper = ShopifyScraper::new(concurrent, 10)?;
        let start_time = Instant::now();
        
        // Scrape products
        let products = scraper.scrape_multiple_products(domain, test_handles.clone()).await?;
        
        let elapsed = start_time.elapsed();
        
        println!("✓ Scraped {} products in {:.3} seconds", products.len(), elapsed.as_secs_f64());
        println!("✓ Average time per product: {:.3} seconds", elapsed.as_secs_f64() / products.len() as f64);
        
        if !products.is_empty() {
            let product = &products[0];
            println!("✓ Sample product: {} (${})", product.title, product.price);
        }
    }
    
    // Test auto-discovery performance
    println!("\n--- Auto-Discovery Performance Test ---");
    let scraper = ShopifyScraper::new(100, 10)?;
    let start_time = Instant::now();
    
    let discovered = scraper.discover_products(domain, 20).await?;
    let elapsed = start_time.elapsed();
    
    println!("✓ Discovered {} products in {:.3} seconds", discovered.len(), elapsed.as_secs_f64());
    if !discovered.is_empty() {
        println!("✓ Sample handles: {:?}", &discovered[..3.min(discovered.len())]);
    }
    
    println!("\n🎉 Benchmark completed!");
    
    Ok(())
}