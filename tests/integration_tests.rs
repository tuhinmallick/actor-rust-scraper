use anyhow::Result;
use shopify_scraper::{ShopifyScraper, ShopifyProduct, ProductVariant};
use chrono::Utc;

#[tokio::test]
async fn test_data_transformation() -> Result<()> {
    // Mock Shopify product data
    let mock_product = shopify_scraper::models::RawShopifyProduct {
        id: 123456789,
        title: "Test Product".to_string(),
        body_html: Some("<p>This is a test product description</p>".to_string()),
        vendor: "Test Vendor".to_string(),
        product_type: "Test Type".to_string(),
        tags: "test, product, example".to_string(),
        available: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        handle: "test-product".to_string(),
        variants: vec![shopify_scraper::models::RawProductVariant {
            id: 987654321,
            title: "Default Title".to_string(),
            price: "29.99".to_string(),
            sku: "TEST-SKU".to_string(),
            inventory_quantity: 100,
            available: true,
            weight: 0.5,
            weight_unit: "kg".to_string(),
        }],
        images: vec![
            shopify_scraper::models::RawProductImage {
                src: "https://example.com/image1.jpg".to_string(),
            },
            shopify_scraper::models::RawProductImage {
                src: "https://example.com/image2.jpg".to_string(),
            },
        ],
    };
    
    // Create scraper instance for transformation testing
    let scraper = ShopifyScraper::new(10, 5)?;
    
    // Test transformation
    let canonical_product = scraper.transform_to_canonical(mock_product);
    
    // Validate canonical format
    assert_eq!(canonical_product.id, "123456789");
    assert_eq!(canonical_product.title, "Test Product");
    assert_eq!(canonical_product.description, "This is a test product description");
    assert_eq!(canonical_product.price, 29.99);
    assert_eq!(canonical_product.currency, "USD");
    assert_eq!(canonical_product.variants.len(), 1);
    assert_eq!(canonical_product.images.len(), 2);
    
    // Validate canonical rules
    assert_eq!(canonical_product.product_type, "Test Type"); // camelCase
    assert!(canonical_product.title_de.is_none()); // English default
    
    Ok(())
}

#[tokio::test]
async fn test_domain_normalization() -> Result<()> {
    let scraper = ShopifyScraper::new(10, 5)?;
    
    // Test domain normalization
    let normalized = scraper.normalize_domain("example.myshopify.com")?;
    assert_eq!(normalized, "https://example.myshopify.com");
    
    let normalized_with_https = scraper.normalize_domain("https://example.myshopify.com")?;
    assert_eq!(normalized_with_https, "https://example.myshopify.com");
    
    let normalized_with_trailing_slash = scraper.normalize_domain("https://example.myshopify.com/")?;
    assert_eq!(normalized_with_trailing_slash, "https://example.myshopify.com");
    
    Ok(())
}

#[tokio::test]
async fn test_shopify_store_detection() -> Result<()> {
    let scraper = ShopifyScraper::new(10, 5)?;
    
    // Test Shopify store detection
    assert!(scraper.is_shopify_store("example.myshopify.com"));
    assert!(scraper.is_shopify_store("https://example.myshopify.com"));
    assert!(scraper.is_shopify_store("store.shopify.com"));
    assert!(!scraper.is_shopify_store("example.com"));
    assert!(!scraper.is_shopify_store("not-shopify.com"));
    
    Ok(())
}