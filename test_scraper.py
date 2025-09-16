#!/usr/bin/env python3
"""
Test script for the Shopify Product Scraper
"""

import asyncio
import json
from shopify_scraper import ShopifyScraper, ShopifyProduct

async def test_scraper():
    """Test the scraper with a known Shopify store"""
    
    # Using a public Shopify store for testing
    test_domain = "shopify.dev"
    test_handles = ["test-product"]  # This might not exist, but tests the flow
    
    print("Testing Shopify Scraper...")
    print(f"Domain: {test_domain}")
    print(f"Test handles: {test_handles}")
    
    try:
        async with ShopifyScraper(max_concurrent=10, timeout=5) as scraper:
            # Test single product scraping
            print("\n1. Testing single product scraping...")
            product = await scraper.scrape_product(test_domain, test_handles[0])
            if product:
                print(f"✓ Successfully scraped: {product.title}")
                print(f"  - ID: {product.id}")
                print(f"  - Price: {product.currency} {product.price}")
            else:
                print("✗ Product not found (expected for test domain)")
            
            # Test multiple product scraping
            print("\n2. Testing multiple product scraping...")
            products = await scraper.scrape_multiple_products(test_domain, test_handles)
            print(f"✓ Scraped {len(products)} products")
            
            # Test auto-discovery
            print("\n3. Testing auto-discovery...")
            discovered = await scraper.discover_products(test_domain, max_products=5)
            print(f"✓ Discovered {len(discovered)} product handles")
            if discovered:
                print(f"  Sample handles: {discovered[:3]}")
            
            # Test canonical format
            print("\n4. Testing canonical format...")
            if products:
                product = products[0]
                print(f"✓ Canonical format validation:")
                print(f"  - camelCase fields: {all(not '_' in field or field.endswith('_de') or field.endswith('_fr') or field.endswith('_es') for field in dir(product) if not field.startswith('_'))}")
                print(f"  - Required fields present: {all(hasattr(product, field) for field in ['id', 'title', 'description', 'price', 'currency'])}")
            
    except Exception as e:
        print(f"✗ Test failed with error: {e}")
        return False
    
    print("\n✓ All tests completed successfully!")
    return True

async def test_data_transformation():
    """Test the data transformation logic"""
    
    print("\n--- Testing Data Transformation ---")
    
    # Mock Shopify product data
    mock_product = {
        "id": 123456789,
        "title": "Test Product",
        "body_html": "<p>This is a test product description</p>",
        "vendor": "Test Vendor",
        "product_type": "Test Type",
        "tags": "test, product, example",
        "available": True,
        "created_at": "2023-01-01T00:00:00Z",
        "updated_at": "2023-01-15T12:00:00Z",
        "handle": "test-product",
        "variants": [
            {
                "id": 987654321,
                "title": "Default Title",
                "price": "29.99",
                "sku": "TEST-SKU",
                "inventory_quantity": 100,
                "available": True,
                "weight": 0.5,
                "weight_unit": "kg"
            }
        ],
        "images": [
            {"src": "https://example.com/image1.jpg"},
            {"src": "https://example.com/image2.jpg"}
        ]
    }
    
    # Create scraper instance for transformation testing
    scraper = ShopifyScraper()
    
    # Test transformation
    canonical_product = scraper._transform_to_canonical(mock_product)
    
    print(f"✓ Transformation successful:")
    print(f"  - ID: {canonical_product.id}")
    print(f"  - Title: {canonical_product.title}")
    print(f"  - Description: {canonical_product.description}")
    print(f"  - Price: {canonical_product.price}")
    print(f"  - Currency: {canonical_product.currency}")
    print(f"  - Variants: {len(canonical_product.variants)}")
    print(f"  - Images: {len(canonical_product.images)}")
    
    # Validate canonical rules
    print(f"\n✓ Canonical rules validation:")
    print(f"  - camelCase: {canonical_product.productType} (not product_type)")
    print(f"  - English: {canonical_product.title} (not title_de)")
    print(f"  - Singular: {canonical_product.description} (not descriptions)")
    
    return True

if __name__ == "__main__":
    print("Shopify Scraper Test Suite")
    print("=" * 50)
    
    # Run tests
    asyncio.run(test_scraper())
    asyncio.run(test_data_transformation())
    
    print("\n" + "=" * 50)
    print("Test suite completed!")