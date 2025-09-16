use crate::models::*;
use crate::schema::*;
use crate::data_wrangling::DataWranglingPipeline;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use tracing::{error, info, warn};
use url::Url;
use moka::future::Cache;

/// Cache entry for failed requests with retry information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheEntry {
    pub url: String,
    pub last_failure: Instant,
    pub failure_count: u32,
    pub last_error: String,
    pub retry_after: Option<Duration>,
}

/// High-performance multi-website scraper with caching and intelligent retry
pub struct MultiWebsiteScraper {
    client: Client,
    semaphore: Arc<Semaphore>,
    timeout: Duration,
    config: ScraperConfig,
    // Cache for failed requests to avoid immediate retries
    failure_cache: Cache<String, CacheEntry>,
    // Cache for successful responses to avoid duplicate requests
    response_cache: Cache<String, Vec<u8>>,
    // Per-domain rate limiting
    domain_limits: Arc<RwLock<HashMap<String, Instant>>>,
    // Performance metrics
    metrics: Arc<RwLock<ScrapingMetrics>>,
    // Data wrangling pipeline for processing scraped data
    data_wrangling_pipeline: DataWranglingPipeline,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct ScrapingMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cached_responses: u64,
    pub retry_attempts: u64,
    pub start_time: Option<Instant>,
    pub total_duration: Option<Duration>,
}

/// Configuration for multi-website scraping
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MultiWebsiteConfig {
    pub websites: Vec<String>,
    pub max_products_per_site: usize,
    pub max_concurrent_per_domain: usize,
    pub global_max_concurrent: usize,
    pub cache_ttl_seconds: u64,
    pub retry_delay_ms: u64,
    pub max_retries: u32,
    pub enable_response_caching: bool,
    pub enable_failure_caching: bool,
    pub rate_limit_per_domain_ms: u64,
    pub timeout_seconds: u64,
}

impl Default for MultiWebsiteConfig {
    fn default() -> Self {
        Self {
            websites: vec![],
            max_products_per_site: 1000,
            max_concurrent_per_domain: 50,
            global_max_concurrent: 200,
            cache_ttl_seconds: 3600, // 1 hour
            retry_delay_ms: 1000,
            max_retries: 3,
            enable_response_caching: true,
            enable_failure_caching: true,
            rate_limit_per_domain_ms: 100, // 100ms between requests per domain
            timeout_seconds: 30,
        }
    }
}

impl MultiWebsiteScraper {
    /// Create a new high-performance multi-website scraper
    pub fn new(config: MultiWebsiteConfig, input: ScraperInput) -> Result<Self> {
        let scraper_config = ScraperConfig {
            input: input.clone(),
            user_agent: "MultiWebsiteScraper/2.0 (Rust)".to_string(),
            rate_limit_delay: config.rate_limit_per_domain_ms,
            max_redirects: 5,
        };

        // Build optimized HTTP client
        let mut client_builder = Client::builder()
            .user_agent(&scraper_config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(scraper_config.max_redirects))
            .pool_max_idle_per_host(50) // Increased connection pooling
            .pool_idle_timeout(Duration::from_secs(90))
            .gzip(true)
            .brotli(true)
            .danger_accept_invalid_certs(true) // For Docker environments with SSL issues
            .danger_accept_invalid_hostnames(true);

        // Only set timeout if it's greater than 0
        if config.timeout_seconds > 0 {
            client_builder = client_builder.timeout(Duration::from_secs(config.timeout_seconds));
        }

        let client = client_builder.build()?;
        let semaphore = Arc::new(Semaphore::new(config.global_max_concurrent));

        // Initialize caches
        let failure_cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .max_capacity(10_000)
            .build();

        let response_cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .max_capacity(50_000)
            .build();

        Ok(Self {
            client,
            semaphore,
            timeout: Duration::from_secs(config.timeout_seconds),
            config: scraper_config,
            failure_cache,
            response_cache,
            domain_limits: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ScrapingMetrics {
                start_time: Some(Instant::now()),
                ..Default::default()
            })),
            data_wrangling_pipeline: DataWranglingPipeline::new(),
        })
    }

    /// Normalize domain to ensure proper format
    fn normalize_domain(&self, domain: &str) -> Result<String> {
        let domain = domain.trim();
        let url = if domain.starts_with("http://") || domain.starts_with("https://") {
            Url::parse(domain)?
        } else {
            Url::parse(&format!("https://{}", domain))?
        };

        Ok(url.to_string().trim_end_matches('/').to_string())
    }

    /// Check if we should rate limit requests to a domain
    async fn should_rate_limit(&self, domain: &str) -> bool {
        let mut limits = self.domain_limits.write().await;
        if let Some(last_request) = limits.get(domain) {
            if last_request.elapsed() < Duration::from_millis(self.config.rate_limit_delay) {
                return true;
            }
        }
        limits.insert(domain.to_string(), Instant::now());
        false
    }

    /// Check if a URL is in the failure cache and should be retried
    async fn should_retry_url(&self, url: &str) -> bool {
        if let Some(entry) = self.failure_cache.get(url).await {
            // Check if enough time has passed since last failure
            if let Some(retry_after) = entry.retry_after {
                if entry.last_failure.elapsed() < retry_after {
                    return false;
                }
            } else {
                // Exponential backoff: 2^failure_count seconds
                let backoff_duration = Duration::from_secs(2_u64.pow(entry.failure_count.min(10)));
                if entry.last_failure.elapsed() < backoff_duration {
                    return false;
                }
            }
        }
        true
    }

    /// Update failure cache with retry information
    async fn update_failure_cache(&self, url: &str, error: &str) {
        let mut entry = self.failure_cache.get(url).await.unwrap_or_else(|| CacheEntry {
            url: url.to_string(),
            last_failure: Instant::now(),
            failure_count: 0,
            last_error: error.to_string(),
            retry_after: None,
        });

        entry.failure_count += 1;
        entry.last_failure = Instant::now();
        entry.last_error = error.to_string();

        // Set retry_after for specific error types
        if error.contains("429") || error.contains("rate limit") {
            entry.retry_after = Some(Duration::from_secs(60));
        } else if error.contains("503") || error.contains("service unavailable") {
            entry.retry_after = Some(Duration::from_secs(30));
        }

        self.failure_cache.insert(url.to_string(), entry).await;
    }

    /// Fetch data with caching and intelligent retry
    async fn fetch_with_cache(&self, url: &str) -> Result<Option<Vec<u8>>> {
        // Check response cache first
        if let Some(cached_data) = self.response_cache.get(url).await {
            let mut metrics = self.metrics.write().await;
            metrics.cached_responses += 1;
            return Ok(Some(cached_data));
        }

        // Check if we should retry this URL
        if !self.should_retry_url(url).await {
            return Ok(None);
        }

        // Rate limiting per domain
        if let Ok(domain) = Url::parse(url) {
            if let Some(host) = domain.host_str() {
                if self.should_rate_limit(host).await {
                    tokio::time::sleep(Duration::from_millis(self.config.rate_limit_delay)).await;
                }
            }
        }

        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
        }

        // Make the request with retries
        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            match self.client.get(url).send().await {
                Ok(response) => {
                    match response.status() {
                        reqwest::StatusCode::OK => {
                            let data = response.bytes().await?.to_vec();
                            
                            // Cache successful response
                            self.response_cache.insert(url.to_string(), data.clone()).await;
                            
                            // Update metrics
                            {
                                let mut metrics = self.metrics.write().await;
                                metrics.successful_requests += 1;
                            }
                            
                            return Ok(Some(data));
                        }
                        reqwest::StatusCode::NOT_FOUND => {
                            warn!("Resource not found: {}", url);
                            return Ok(None);
                        }
                        reqwest::StatusCode::TOO_MANY_REQUESTS => {
                            let error_msg = format!("Rate limited: {}", url);
                            self.update_failure_cache(url, &error_msg).await;
                            
                            if attempts < max_attempts - 1 {
                                let delay = Duration::from_millis(1000 * (attempts + 1) as u64);
                                tokio::time::sleep(delay).await;
                                attempts += 1;
                                continue;
                            }
                            return Err(anyhow!("Rate limited after {} attempts", max_attempts));
                        }
                        status => {
                            let error_msg = format!("HTTP {} for {}", status, url);
                            self.update_failure_cache(url, &error_msg).await;
                            return Err(anyhow!("HTTP error: {}", status));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Request failed: {}", e);
                    self.update_failure_cache(url, &error_msg).await;
                    
                    if attempts < max_attempts - 1 {
                        let delay = Duration::from_millis(1000 * (attempts + 1) as u64);
                        tokio::time::sleep(delay).await;
                        attempts += 1;
                        continue;
                    }
                    return Err(anyhow!("Request failed after {} attempts: {}", max_attempts, e));
                }
            }
        }

        Err(anyhow!("Max retries exceeded"))
    }

    /// Discover products from a single website
    async fn discover_products_from_website(&self, domain: &str, max_products: usize) -> Result<Vec<String>> {
        let domain = self.normalize_domain(domain)?;
        
        // Try multiple endpoints for product discovery with pagination
        let limit = self.config.input.pagination.limit;
        let page = self.config.input.pagination.page;
        let enable_pagination = self.config.input.pagination.enable_pagination;
        
        let urls_to_try = if enable_pagination {
            vec![
                format!("{}/products.json?limit={}&page={}", domain, limit, page),
            ]
        } else {
            vec![
                format!("{}/products.json?limit={}", domain, limit),
            ]
        };

        for url in urls_to_try {
            if let Some(data) = self.fetch_with_cache(&url).await? {
                if url.contains(".json") {
                    if let Ok(json_data) = serde_json::from_slice::<serde_json::Value>(&data) {
                        if let Some(products) = json_data.get("products").and_then(|p| p.as_array()) {
                            let handles: Vec<String> = products
                                .iter()
                                .take(max_products)
                                .filter_map(|p| p.get("handle").and_then(|h| h.as_str()))
                                .map(|s| s.to_string())
                                .collect();
                            if !handles.is_empty() {
                                return Ok(handles);
                            }
                        }
                    }
                } else if url.contains(".xml") {
                    if let Ok(content) = String::from_utf8(data) {
                        let handle_pattern = regex::Regex::new(r"/products/([^/]+)")?;
                        let handles: Vec<String> = handle_pattern
                            .captures_iter(&content)
                            .take(max_products)
                            .filter_map(|cap| cap.get(1))
                            .map(|m| m.as_str().to_string())
                            .collect();
                        if !handles.is_empty() {
                            return Ok(handles);
                        }
                    }
                }
            }
        }

        warn!("Could not discover products from {}", domain);
        Ok(vec![])
    }

    /// Scrape products from multiple websites in parallel
    pub async fn scrape_multiple_websites(&self, config: MultiWebsiteConfig) -> Result<HashMap<String, Vec<ShopifyProduct>>> {
        info!("Starting multi-website scraping for {} websites", config.websites.len());
        
        let start_time = Instant::now();
        let mut results = HashMap::new();

        // Create tasks for each website
        let tasks: Vec<_> = config.websites.clone()
            .into_iter()
            .map(|website| {
                let mut scraper = self.clone();
                let config = config.clone();
                tokio::spawn(async move {
                    scraper.scrape_single_website(&website, config.max_products_per_site).await
                })
            })
            .collect();

        // Execute all tasks in parallel
        let task_results = join_all(tasks).await;

        // Collect results
        for (i, result) in task_results.into_iter().enumerate() {
            match result {
                Ok(Ok((website, products))) => {
                    info!("Scraped {} products from {}", products.len(), website);
                    results.insert(website, products);
                }
                Ok(Err(e)) => {
                    error!("Failed to scrape website {}: {}", config.websites[i], e);
                }
                Err(e) => {
                    error!("Task error for website {}: {}", config.websites[i], e);
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!("Multi-website scraping completed in {:.3} seconds", elapsed.as_secs_f64());

        // Update final metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_duration = Some(elapsed);
        }

        self.print_performance_metrics().await;

        Ok(results)
    }

    /// Scrape products from a single website
    async fn scrape_single_website(&mut self, domain: &str, max_products: usize) -> Result<(String, Vec<ShopifyProduct>)> {
        let domain = self.normalize_domain(domain)?;
        
        // Discover products
        let product_handles = self.discover_products_from_website(&domain, max_products).await?;
        
        if product_handles.is_empty() {
            warn!("No products found for {}", domain);
            return Ok((domain, vec![]));
        }

        info!("Discovered {} products from {}", product_handles.len(), domain);

        // Scrape products in parallel
        let tasks: Vec<_> = product_handles
            .into_iter()
            .map(|handle| {
                let scraper = self.clone();
                let domain = domain.clone();
                tokio::spawn(async move {
                    scraper.scrape_single_product(&domain, &handle).await
                })
            })
            .collect();

        let results = join_all(tasks).await;
        let mut raw_products = Vec::new();

        for result in results {
            match result {
                Ok(Ok(Some(product))) => raw_products.push(product),
                Ok(Ok(None)) => {} // Product not found or filtered out
                Ok(Err(e)) => error!("Scraping error: {}", e),
                Err(e) => error!("Task error: {}", e),
            }
        }

        // Process products through data wrangling pipeline
        let processed_products = if !raw_products.is_empty() {
            info!("Processing {} raw products through data wrangling pipeline", raw_products.len());
            match self.data_wrangling_pipeline.process_products(&raw_products, &domain).await {
                Ok(products) => {
                    info!("Data wrangling completed: {} products processed", products.len());
                    products
                }
                Err(e) => {
                    error!("Data wrangling failed: {}", e);
                    raw_products // Fallback to raw products
                }
            }
        } else {
            raw_products
        };

        Ok((domain, processed_products))
    }

    /// Scrape a single product
    async fn scrape_single_product(&self, domain: &str, product_handle: &str) -> Result<Option<ShopifyProduct>> {
        let url = format!("{}/products/{}.json", domain, product_handle);
        
        if let Some(data) = self.fetch_with_cache(&url).await? {
            if let Ok(api_response) = serde_json::from_slice::<ShopifyApiResponse>(&data) {
                let raw_product = api_response.product;
                
                // Transform to canonical format (simplified version)
                let product = ShopifyProduct {
                    id: raw_product.id.to_string(),
                    title: raw_product.title,
                    description: raw_product.body_html.unwrap_or_default(),
                    price: raw_product.variants.first()
                        .and_then(|v| v.price.parse().ok())
                        .unwrap_or(0.0),
                    currency: "USD".to_string(),
                    availability: raw_product.available.unwrap_or(false),
                    vendor: raw_product.vendor,
                    product_type: raw_product.product_type,
                    tags: raw_product.tags.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    images: raw_product.images.into_iter()
                        .map(|img| ProductImage {
                            src: img.src,
                            alt: img.alt,
                            width: img.width,
                            height: img.height,
                            position: img.position,
                        })
                        .collect(),
                    variants: raw_product.variants.into_iter()
                        .map(|v| ProductVariant {
                            id: v.id.to_string(),
                            title: v.title,
                            price: v.price.parse().unwrap_or(0.0),
                            sku: v.sku,
                            inventory_quantity: v.inventory_quantity.unwrap_or(0),
                            available: v.available.unwrap_or(false),
                            weight: v.weight.unwrap_or(0.0),
                            weight_unit: v.weight_unit.unwrap_or("kg".to_string()),
                            option1: v.option1,
                            option2: v.option2,
                            option3: v.option3,
                            barcode: v.barcode,
                            compare_at_price: v.compare_at_price.and_then(|p| p.parse().ok()),
                            fulfillment_service: v.fulfillment_service,
                            inventory_management: v.inventory_management,
                            inventory_policy: v.inventory_policy,
                            requires_shipping: v.requires_shipping,
                            taxable: v.taxable,
                            tax_code: v.tax_code,
                        })
                        .collect(),
                    created_at: raw_product.created_at,
                    updated_at: raw_product.updated_at,
                    handle: raw_product.handle.clone(),
                    url: format!("{}/products/{}", domain, raw_product.handle),
                    seo_data: None,
                    analytics_data: None,
                    related_products: None,
                    reviews: None,
                    collections: None,
                    custom_fields: None,
                    shipping_info: None,
                    return_policy: None,
                    warranty: None,
                    title_de: None,
                    title_fr: None,
                    title_es: None,
                    description_de: None,
                    description_fr: None,
                    description_es: None,
                };
                
                return Ok(Some(product));
            }
        }
        
        Ok(None)
    }

    /// Print performance metrics
    async fn print_performance_metrics(&self) {
        let metrics = self.metrics.read().await;
        info!("=== Performance Metrics ===");
        info!("Total requests: {}", metrics.total_requests);
        info!("Successful requests: {}", metrics.successful_requests);
        info!("Failed requests: {}", metrics.failed_requests);
        info!("Cached responses: {}", metrics.cached_responses);
        info!("Retry attempts: {}", metrics.retry_attempts);
        
        if let Some(duration) = metrics.total_duration {
            let requests_per_second = metrics.total_requests as f64 / duration.as_secs_f64();
            info!("Total duration: {:.3} seconds", duration.as_secs_f64());
            info!("Requests per second: {:.2}", requests_per_second);
        }
        
        info!("Success rate: {:.2}%", 
              (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0);
    }
}

impl Clone for MultiWebsiteScraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
            timeout: self.timeout,
            config: self.config.clone(),
            failure_cache: self.failure_cache.clone(),
            response_cache: self.response_cache.clone(),
            domain_limits: self.domain_limits.clone(),
            metrics: self.metrics.clone(),
            data_wrangling_pipeline: self.data_wrangling_pipeline.clone(),
        }
    }
}
