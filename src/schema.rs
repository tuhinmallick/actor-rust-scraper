use serde::{Deserialize, Serialize};

/// Apify-compatible input schema for Shopify Lightning Scraper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperInput {
    /// Shopify store domain (required for single website mode)
    pub domain: String,
    
    /// Multiple websites to scrape (for multi-website mode)
    #[serde(default)]
    pub websites: Vec<String>,
    
    /// Enable multi-website mode
    #[serde(default = "default_false")]
    pub multi_website_mode: bool,
    
    /// Specific product handles to scrape
    #[serde(default)]
    pub product_handles: Vec<String>,
    
    /// Auto-discover products from store
    #[serde(default = "default_true")]
    pub auto_discover: bool,
    
    /// Maximum products to scrape per website
    #[serde(default = "default_max_products")]
    pub max_products: usize,
    
    /// Maximum concurrent requests per domain
    #[serde(default = "default_concurrent")]
    pub max_concurrent: usize,
    
    /// Global maximum concurrent requests across all websites
    #[serde(default = "default_global_concurrent")]
    pub global_max_concurrent: usize,
    
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
    
    /// Caching and retry settings
    #[serde(default)]
    pub caching: CachingSettings,
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
    
    /// Filter by availability
    #[serde(default)]
    pub availability: Option<bool>,
    
    /// Search in product title and description
    #[serde(default)]
    pub search_query: Option<String>,
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
    
    /// Include custom fields
    #[serde(default = "default_false")]
    pub include_custom_fields: bool,
}

/// Caching and retry settings for optimal performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingSettings {
    /// Enable response caching
    #[serde(default = "default_true")]
    pub enable_response_caching: bool,
    
    /// Enable failure caching to avoid immediate retries
    #[serde(default = "default_true")]
    pub enable_failure_caching: bool,
    
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
    
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    
    /// Base retry delay in milliseconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
    
    /// Rate limit delay per domain in milliseconds
    #[serde(default = "default_rate_limit_delay")]
    pub rate_limit_per_domain_ms: u64,
    
    /// Enable exponential backoff for retries
    #[serde(default = "default_true")]
    pub enable_exponential_backoff: bool,
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
    
    /// Retry failed requests
    #[serde(default = "default_true")]
    pub enable_retries: bool,
    
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
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
            websites: Vec::new(),
            multi_website_mode: false,
            product_handles: Vec::new(),
            auto_discover: true,
            max_products: 1000,
            max_concurrent: 100,
            global_max_concurrent: 200,
            timeout_seconds: 30,
            output_format: OutputFormat::Json,
            filters: ProductFilters::default(),
            extraction: ExtractionOptions::default(),
            performance: PerformanceSettings::default(),
            caching: CachingSettings::default(),
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
            availability: None,
            search_query: None,
        }
    }
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            include_images: true,
            include_variants: true,
            include_custom_fields: false,
        }
    }
}

impl Default for CachingSettings {
    fn default() -> Self {
        Self {
            enable_response_caching: true,
            enable_failure_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_retries: 3,
            retry_delay_ms: 1000,
            rate_limit_per_domain_ms: 100,
            enable_exponential_backoff: true,
        }
    }
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            enable_connection_pooling: true,
            enable_compression: true,
            enable_retries: true,
            max_retries: 3,
        }
    }
}

// Default value functions
fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_products() -> usize { 1000 }
fn default_concurrent() -> usize { 100 }
fn default_global_concurrent() -> usize { 200 }
fn default_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_cache_ttl() -> u64 { 3600 }
fn default_retry_delay() -> u64 { 1000 }
fn default_rate_limit_delay() -> u64 { 100 }