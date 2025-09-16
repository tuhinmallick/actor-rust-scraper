use serde::{Deserialize, Serialize};

/// Apify-compatible input schema for Shopify Lightning Scraper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperInput {
    /// Shopify store domain (required)
    pub domain: String,
    
    /// Specific product handles to scrape
    #[serde(default)]
    pub product_handles: Vec<String>,
    
    /// Auto-discover products from store
    #[serde(default = "default_true")]
    pub auto_discover: bool,
    
    /// Maximum products to scrape
    #[serde(default = "default_max_products")]
    pub max_products: usize,
    
    /// Maximum concurrent requests
    #[serde(default = "default_concurrent")]
    pub max_concurrent: usize,
    
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    /// Output format
    #[serde(default)]
    pub output_format: OutputFormat,
    
    /// Advanced filtering options
    #[serde(default)]
    pub filters: ProductFilters,
    
    /// Data extraction options
    #[serde(default)]
    pub extraction: ExtractionOptions,
    
    /// Performance optimization settings
    #[serde(default)]
    pub performance: PerformanceSettings,
}

/// Product filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFilters {
    /// Filter by minimum price
    #[serde(default)]
    pub min_price: Option<f64>,
    
    /// Filter by maximum price
    #[serde(default)]
    pub max_price: Option<f64>,
    
    /// Filter by currency
    #[serde(default)]
    pub currency: Option<String>,
    
    /// Filter by vendor
    #[serde(default)]
    pub vendors: Vec<String>,
    
    /// Filter by product type
    #[serde(default)]
    pub product_types: Vec<String>,
    
    /// Filter by tags (any of these tags)
    #[serde(default)]
    pub tags_any: Vec<String>,
    
    /// Filter by tags (all of these tags)
    #[serde(default)]
    pub tags_all: Vec<String>,
    
    /// Exclude products with these tags
    #[serde(default)]
    pub tags_exclude: Vec<String>,
    
    /// Filter by availability
    #[serde(default)]
    pub availability: Option<bool>,
    
    /// Filter by minimum inventory
    #[serde(default)]
    pub min_inventory: Option<i32>,
    
    /// Filter by creation date (ISO 8601)
    #[serde(default)]
    pub created_after: Option<String>,
    
    /// Filter by creation date (ISO 8601)
    #[serde(default)]
    pub created_before: Option<String>,
    
    /// Filter by update date (ISO 8601)
    #[serde(default)]
    pub updated_after: Option<String>,
    
    /// Filter by update date (ISO 8601)
    #[serde(default)]
    pub updated_before: Option<String>,
    
    /// Search in product title and description
    #[serde(default)]
    pub search_query: Option<String>,
    
    /// Regex pattern for product handles
    #[serde(default)]
    pub handle_pattern: Option<String>,
}

/// Data extraction options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionOptions {
    /// Include product images
    #[serde(default = "default_true")]
    pub include_images: bool,
    
    /// Include product variants
    #[serde(default = "default_true")]
    pub include_variants: bool,
    
    /// Include SEO data (meta descriptions, etc.)
    #[serde(default = "default_false")]
    pub include_seo: bool,
    
    /// Include analytics data
    #[serde(default = "default_false")]
    pub include_analytics: bool,
    
    /// Include related products
    #[serde(default = "default_false")]
    pub include_related: bool,
    
    /// Include reviews and ratings
    #[serde(default = "default_false")]
    pub include_reviews: bool,
    
    /// Include inventory levels
    #[serde(default = "default_true")]
    pub include_inventory: bool,
    
    /// Include pricing history
    #[serde(default = "default_false")]
    pub include_pricing_history: bool,
    
    /// Include product recommendations
    #[serde(default = "default_false")]
    pub include_recommendations: bool,
    
    /// Include custom fields
    #[serde(default = "default_false")]
    pub include_custom_fields: bool,
    
    /// Include product collections
    #[serde(default = "default_false")]
    pub include_collections: bool,
    
    /// Include product bundles
    #[serde(default = "default_false")]
    pub include_bundles: bool,
    
    /// Include shipping information
    #[serde(default = "default_false")]
    pub include_shipping: bool,
    
    /// Include return policy
    #[serde(default = "default_false")]
    pub include_return_policy: bool,
    
    /// Include warranty information
    #[serde(default = "default_false")]
    pub include_warranty: bool,
}

/// Performance optimization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Enable connection pooling
    #[serde(default = "default_true")]
    pub enable_connection_pooling: bool,
    
    /// Enable compression (gzip/brotli)
    #[serde(default = "default_true")]
    pub enable_compression: bool,
    
    /// Enable HTTP/2
    #[serde(default = "default_true")]
    pub enable_http2: bool,
    
    /// Enable keep-alive connections
    #[serde(default = "default_true")]
    pub enable_keep_alive: bool,
    
    /// Retry failed requests
    #[serde(default = "default_true")]
    pub enable_retries: bool,
    
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
    
    /// Enable request deduplication
    #[serde(default = "default_true")]
    pub enable_deduplication: bool,
    
    /// Enable response caching
    #[serde(default = "default_false")]
    pub enable_caching: bool,
    
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

/// Output format options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    JsonL,
    Csv,
    Xml,
    Parquet,
}

/// Scraper configuration
#[derive(Debug, Clone)]
pub struct ScraperConfig {
    pub input: ScraperInput,
    pub user_agent: String,
    pub rate_limit_delay: u64,
    pub max_redirects: usize,
}

impl Default for ScraperInput {
    fn default() -> Self {
        Self {
            domain: String::new(),
            product_handles: Vec::new(),
            auto_discover: true,
            max_products: 1000,
            max_concurrent: 100,
            timeout_seconds: 30,
            output_format: OutputFormat::Json,
            filters: ProductFilters::default(),
            extraction: ExtractionOptions::default(),
            performance: PerformanceSettings::default(),
        }
    }
}

impl Default for ProductFilters {
    fn default() -> Self {
        Self {
            min_price: None,
            max_price: None,
            currency: None,
            vendors: Vec::new(),
            product_types: Vec::new(),
            tags_any: Vec::new(),
            tags_all: Vec::new(),
            tags_exclude: Vec::new(),
            availability: None,
            min_inventory: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            search_query: None,
            handle_pattern: None,
        }
    }
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            include_images: true,
            include_variants: true,
            include_seo: false,
            include_analytics: false,
            include_related: false,
            include_reviews: false,
            include_inventory: true,
            include_pricing_history: false,
            include_recommendations: false,
            include_custom_fields: false,
            include_collections: false,
            include_bundles: false,
            include_shipping: false,
            include_return_policy: false,
            include_warranty: false,
        }
    }
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            enable_connection_pooling: true,
            enable_compression: true,
            enable_http2: true,
            enable_keep_alive: true,
            enable_retries: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_deduplication: true,
            enable_caching: false,
            cache_ttl_seconds: 3600,
        }
    }
}

// Default value functions
fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_products() -> usize { 1000 }
fn default_concurrent() -> usize { 100 }
fn default_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_retry_delay() -> u64 { 1000 }
fn default_cache_ttl() -> u64 { 3600 }