# High-Performance Shopify Product Scraper

A blazingly fast, parallel Shopify product scraper that fetches product data from `/product.json` endpoints and transforms it according to canonical key rules.

## Features

- ⚡ **Millisecond Performance**: Parallel processing with asyncio for ultra-fast scraping
- 🔄 **Auto-Discovery**: Automatically discover products from Shopify stores
- 📊 **Canonical Format**: Transforms data according to strict canonical key rules:
  - Language: English
  - Case: camelCase
  - Specificity: Clear but Concise
  - Singularity & Plurality: Strict Rule
- 🌍 **Multi-Language Support**: Handles language-specific fields with suffixing convention
- 📈 **High Concurrency**: Configurable concurrent requests (default: 100)
- 🛡️ **Robust Error Handling**: Comprehensive error handling and timeout management
- 📋 **Multiple Output Formats**: JSON and CSV output support

## Installation

```bash
pip install -r requirements.txt
```

## Quick Start

### Scrape Specific Products
```bash
python shopify_scraper.py store.myshopify.com --products product-handle-1 product-handle-2
```

### Auto-Discover and Scrape All Products
```bash
python shopify_scraper.py store.myshopify.com --discover --max-products 50
```

### High-Performance Scraping
```bash
python shopify_scraper.py store.myshopify.com --discover --concurrent 200 --timeout 5
```

## Usage Examples

### Basic Usage
```bash
# Scrape specific products
python shopify_scraper.py example.myshopify.com --products "awesome-t-shirt" "cool-hoodie"

# Auto-discover products (up to 100)
python shopify_scraper.py example.myshopify.com --discover

# Limit discovered products
python shopify_scraper.py example.myshopify.com --discover --max-products 25
```

### Advanced Usage
```bash
# High concurrency for maximum speed
python shopify_scraper.py example.myshopify.com --discover --concurrent 500

# Custom timeout for slow stores
python shopify_scraper.py example.myshopify.com --discover --timeout 30

# CSV output format
python shopify_scraper.py example.myshopify.com --discover --output csv
```

## Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--products` | `-p` | Specific product handles to scrape | None |
| `--discover` | `-d` | Auto-discover products from store | False |
| `--max-products` | `-m` | Maximum products to scrape | 100 |
| `--output` | `-o` | Output format (json/csv) | json |
| `--concurrent` | `-c` | Max concurrent requests | 100 |
| `--timeout` | `-t` | Request timeout in seconds | 10 |

## Canonical Data Format

The scraper transforms Shopify product data into a canonical format following these immutable rules:

### Core Fields
- `id`: Product ID
- `title`: Product title (English)
- `description`: Product description (English)
- `price`: Product price
- `currency`: Currency code
- `availability`: Availability status
- `vendor`: Product vendor
- `productType`: Product type
- `tags`: Product tags array
- `images`: Product images array
- `variants`: Product variants array
- `createdAt`: Creation timestamp
- `updatedAt`: Last update timestamp
- `handle`: Product handle

### Language-Specific Fields
Following the suffixing convention:
- `title_de`: German title
- `title_fr`: French title
- `title_es`: Spanish title
- `description_de`: German description
- `description_fr`: French description
- `description_es`: Spanish description

### Variant Structure
```json
{
  "id": "variant_id",
  "title": "variant_title",
  "price": 29.99,
  "sku": "SKU123",
  "inventoryQuantity": 100,
  "available": true,
  "weight": 0.5,
  "weightUnit": "kg"
}
```

## Performance Optimization

### Concurrency Settings
- **Conservative**: `--concurrent 50` (for rate-limited stores)
- **Balanced**: `--concurrent 100` (default)
- **Aggressive**: `--concurrent 200-500` (for high-performance needs)

### Timeout Settings
- **Fast stores**: `--timeout 5`
- **Standard**: `--timeout 10` (default)
- **Slow stores**: `--timeout 30`

## Error Handling

The scraper includes comprehensive error handling:
- HTTP status code handling
- Timeout management
- Network error recovery
- Invalid product handling
- Rate limiting protection

## Output Examples

### JSON Output
```json
[
  {
    "id": "123456789",
    "title": "Awesome T-Shirt",
    "description": "A really awesome t-shirt",
    "price": 29.99,
    "currency": "USD",
    "availability": true,
    "vendor": "Awesome Brand",
    "productType": "Apparel",
    "tags": ["clothing", "t-shirt", "casual"],
    "images": ["https://cdn.shopify.com/image1.jpg"],
    "variants": [...],
    "createdAt": "2023-01-01T00:00:00Z",
    "updatedAt": "2023-01-15T12:00:00Z",
    "handle": "awesome-t-shirt"
  }
]
```

### CSV Output
```csv
id,title,description,price,currency,availability,vendor,productType,tags,images,variants,createdAt,updatedAt,handle
123456789,"Awesome T-Shirt","A really awesome t-shirt",29.99,USD,true,"Awesome Brand",Apparel,"[""clothing"",""t-shirt"",""casual""]","[""https://cdn.shopify.com/image1.jpg""]","[{...}]","2023-01-01T00:00:00Z","2023-01-15T12:00:00Z","awesome-t-shirt"
```

## License

MIT License - Feel free to use and modify as needed.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Support

For issues and questions, please open an issue on the repository.