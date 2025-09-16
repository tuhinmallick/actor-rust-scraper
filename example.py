#!/usr/bin/env python3
"""
Example usage of the Shopify Product Scraper
"""

import asyncio
import json
from shopify_scraper import ShopifyScraper

async def example_usage():
    """Example of how to use the Shopify scraper programmatically"""
    
    # Example domain (replace with actual Shopify store)
    domain = "example.myshopify.com"
    
    # Example product handles (replace with actual product handles)
    product_handles = [
        "awesome-t-shirt",
        "cool-hoodie",
        "amazing-jeans"
    ]
    
    print(f"Scraping products from {domain}")
    print(f"Product handles: {product_handles}")
    
    # Use the scraper
    async with ShopifyScraper(max_concurrent=50, timeout=10) as scraper:
        # Scrape multiple products in parallel
        products = await scraper.scrape_multiple_products(domain, product_handles)
        
        print(f"\nScraped {len(products)} products:")
        
        for product in products:
            print(f"\n--- {product.title} ---")
            print(f"ID: {product.id}")
            print(f"Price: {product.currency} {product.price}")
            print(f"Vendor: {product.vendor}")
            print(f"Available: {product.availability}")
            print(f"Variants: {len(product.variants)}")
            print(f"Images: {len(product.images)}")
    
    # Example of auto-discovery
    print(f"\n--- Auto-Discovery Example ---")
    async with ShopifyScraper() as scraper:
        discovered_handles = await scraper.discover_products(domain, max_products=10)
        print(f"Discovered {len(discovered_handles)} product handles:")
        for handle in discovered_handles[:5]:  # Show first 5
            print(f"  - {handle}")

if __name__ == "__main__":
    asyncio.run(example_usage())