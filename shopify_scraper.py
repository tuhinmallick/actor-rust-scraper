#!/usr/bin/env python3
"""
High-Performance Shopify Product Scraper

A parallel, millisecond-fast scraper that fetches product data from Shopify stores
and transforms it according to canonical key rules.
"""

import asyncio
import aiohttp
import json
import re
import time
from typing import Dict, List, Optional, Any, Union
from urllib.parse import urljoin, urlparse
from dataclasses import dataclass, asdict
import logging
from concurrent.futures import ThreadPoolExecutor
import argparse
import sys

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class ShopifyProduct:
    """Canonical product data structure following the four immutable rules"""
    id: str
    title: str
    description: str
    price: float
    currency: str
    availability: str
    vendor: str
    productType: str
    tags: List[str]
    images: List[str]
    variants: List[Dict[str, Any]]
    createdAt: str
    updatedAt: str
    handle: str
    # Language-specific fields follow suffixing convention
    title_de: Optional[str] = None
    title_fr: Optional[str] = None
    title_es: Optional[str] = None
    description_de: Optional[str] = None
    description_fr: Optional[str] = None
    description_es: Optional[str] = None

class ShopifyScraper:
    """High-performance parallel Shopify product scraper"""
    
    def __init__(self, max_concurrent: int = 100, timeout: int = 10):
        self.max_concurrent = max_concurrent
        self.timeout = timeout
        self.session: Optional[aiohttp.ClientSession] = None
        self.semaphore = asyncio.Semaphore(max_concurrent)
        
    async def __aenter__(self):
        """Async context manager entry"""
        connector = aiohttp.TCPConnector(limit=self.max_concurrent, limit_per_host=20)
        timeout = aiohttp.ClientTimeout(total=self.timeout)
        self.session = aiohttp.ClientSession(
            connector=connector,
            timeout=timeout,
            headers={
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
                'Accept': 'application/json',
                'Accept-Language': 'en-US,en;q=0.9'
            }
        )
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        if self.session:
            await self.session.close()
    
    def _normalize_domain(self, domain: str) -> str:
        """Normalize domain to ensure proper format"""
        domain = domain.strip()
        if not domain.startswith(('http://', 'https://')):
            domain = f"https://{domain}"
        return domain.rstrip('/')
    
    def _is_shopify_store(self, domain: str) -> bool:
        """Quick check if domain appears to be a Shopify store"""
        shopify_indicators = [
            'myshopify.com',
            'shopify',
            'cdn.shopify.com'
        ]
        return any(indicator in domain.lower() for indicator in shopify_indicators)
    
    async def _fetch_product_data(self, domain: str, product_handle: str) -> Optional[Dict[str, Any]]:
        """Fetch product data from Shopify product.json endpoint"""
        async with self.semaphore:
            try:
                url = f"{domain}/products/{product_handle}.json"
                async with self.session.get(url) as response:
                    if response.status == 200:
                        data = await response.json()
                        return data.get('product')
                    elif response.status == 404:
                        logger.warning(f"Product not found: {product_handle}")
                        return None
                    else:
                        logger.error(f"HTTP {response.status} for {url}")
                        return None
            except asyncio.TimeoutError:
                logger.error(f"Timeout fetching {product_handle}")
                return None
            except Exception as e:
                logger.error(f"Error fetching {product_handle}: {e}")
                return None
    
    def _transform_to_canonical(self, raw_product: Dict[str, Any]) -> ShopifyProduct:
        """Transform raw Shopify product data to canonical format"""
        
        # Extract basic product information
        product_id = str(raw_product.get('id', ''))
        title = raw_product.get('title', '')
        description = raw_product.get('body_html', '').replace('<p>', '').replace('</p>', '')
        
        # Handle pricing
        variants = raw_product.get('variants', [])
        if variants:
            price = float(variants[0].get('price', 0))
            currency = variants[0].get('currency', 'USD')
        else:
            price = 0.0
            currency = 'USD'
        
        # Extract images
        images = []
        for image in raw_product.get('images', []):
            if isinstance(image, dict):
                images.append(image.get('src', ''))
            else:
                images.append(str(image))
        
        # Transform variants to canonical format
        canonical_variants = []
        for variant in variants:
            canonical_variant = {
                'id': str(variant.get('id', '')),
                'title': variant.get('title', ''),
                'price': float(variant.get('price', 0)),
                'sku': variant.get('sku', ''),
                'inventoryQuantity': variant.get('inventory_quantity', 0),
                'available': variant.get('available', False),
                'weight': variant.get('weight', 0),
                'weightUnit': variant.get('weight_unit', 'kg')
            }
            canonical_variants.append(canonical_variant)
        
        # Create canonical product
        canonical_product = ShopifyProduct(
            id=product_id,
            title=title,
            description=description,
            price=price,
            currency=currency,
            availability=raw_product.get('available', False),
            vendor=raw_product.get('vendor', ''),
            productType=raw_product.get('product_type', ''),
            tags=raw_product.get('tags', []),
            images=images,
            variants=canonical_variants,
            createdAt=raw_product.get('created_at', ''),
            updatedAt=raw_product.get('updated_at', ''),
            handle=raw_product.get('handle', '')
        )
        
        return canonical_product
    
    async def scrape_product(self, domain: str, product_handle: str) -> Optional[ShopifyProduct]:
        """Scrape a single product and return canonical format"""
        domain = self._normalize_domain(domain)
        
        if not self._is_shopify_store(domain):
            logger.warning(f"Domain {domain} may not be a Shopify store")
        
        raw_data = await self._fetch_product_data(domain, product_handle)
        if raw_data:
            return self._transform_to_canonical(raw_data)
        return None
    
    async def scrape_multiple_products(self, domain: str, product_handles: List[str]) -> List[ShopifyProduct]:
        """Scrape multiple products in parallel"""
        domain = self._normalize_domain(domain)
        
        logger.info(f"Scraping {len(product_handles)} products from {domain}")
        start_time = time.time()
        
        # Create tasks for parallel execution
        tasks = [
            self.scrape_product(domain, handle) 
            for handle in product_handles
        ]
        
        # Execute all tasks in parallel
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # Filter out None results and exceptions
        products = []
        for i, result in enumerate(results):
            if isinstance(result, ShopifyProduct):
                products.append(result)
            elif isinstance(result, Exception):
                logger.error(f"Error scraping {product_handles[i]}: {result}")
        
        elapsed_time = time.time() - start_time
        logger.info(f"Scraped {len(products)} products in {elapsed_time:.3f} seconds")
        
        return products
    
    async def discover_products(self, domain: str, max_products: int = 100) -> List[str]:
        """Discover product handles from Shopify store"""
        domain = self._normalize_domain(domain)
        
        try:
            # Try to get products from collections or sitemap
            urls_to_try = [
                f"{domain}/collections/all/products.json",
                f"{domain}/products.json",
                f"{domain}/sitemap_products_1.xml"
            ]
            
            for url in urls_to_try:
                try:
                    async with self.session.get(url) as response:
                        if response.status == 200:
                            if url.endswith('.json'):
                                data = await response.json()
                                products = data.get('products', [])
                                handles = [p.get('handle') for p in products[:max_products]]
                                return [h for h in handles if h]
                            elif url.endswith('.xml'):
                                # Parse XML sitemap for product handles
                                content = await response.text()
                                handles = re.findall(r'/products/([^"]+)', content)
                                return handles[:max_products]
                except Exception as e:
                    logger.debug(f"Failed to fetch {url}: {e}")
                    continue
            
            logger.warning("Could not discover products automatically")
            return []
            
        except Exception as e:
            logger.error(f"Error discovering products: {e}")
            return []

def format_output(products: List[ShopifyProduct], output_format: str = 'json') -> str:
    """Format products according to specified output format"""
    if output_format == 'json':
        return json.dumps([asdict(product) for product in products], indent=2, ensure_ascii=False)
    elif output_format == 'csv':
        if not products:
            return ""
        
        # Get all possible fields
        all_fields = set()
        for product in products:
            all_fields.update(asdict(product).keys())
        
        fields = sorted(list(all_fields))
        
        # Create CSV header
        csv_lines = [','.join(fields)]
        
        # Add data rows
        for product in products:
            product_dict = asdict(product)
            row = []
            for field in fields:
                value = product_dict.get(field, '')
                if isinstance(value, (list, dict)):
                    value = json.dumps(value)
                row.append(f'"{str(value).replace('"', '""')}"')
            csv_lines.append(','.join(row))
        
        return '\n'.join(csv_lines)
    else:
        return str(products)

async def main():
    """Main CLI interface"""
    parser = argparse.ArgumentParser(description='High-Performance Shopify Product Scraper')
    parser.add_argument('domain', help='Shopify store domain (e.g., store.myshopify.com)')
    parser.add_argument('--products', '-p', nargs='+', help='Specific product handles to scrape')
    parser.add_argument('--discover', '-d', action='store_true', help='Auto-discover products')
    parser.add_argument('--max-products', '-m', type=int, default=100, help='Maximum products to scrape')
    parser.add_argument('--output', '-o', choices=['json', 'csv'], default='json', help='Output format')
    parser.add_argument('--concurrent', '-c', type=int, default=100, help='Max concurrent requests')
    parser.add_argument('--timeout', '-t', type=int, default=10, help='Request timeout in seconds')
    
    args = parser.parse_args()
    
    # Determine product handles
    product_handles = []
    if args.products:
        product_handles = args.products
    elif args.discover:
        async with ShopifyScraper(args.concurrent, args.timeout) as scraper:
            product_handles = await scraper.discover_products(args.domain, args.max_products)
            if not product_handles:
                print("No products discovered. Exiting.")
                return
    else:
        print("Error: Must specify either --products or --discover")
        return
    
    # Scrape products
    async with ShopifyScraper(args.concurrent, args.timeout) as scraper:
        products = await scraper.scrape_multiple_products(args.domain, product_handles)
        
        if products:
            output = format_output(products, args.output)
            print(output)
        else:
            print("No products found.")

if __name__ == "__main__":
    asyncio.run(main())