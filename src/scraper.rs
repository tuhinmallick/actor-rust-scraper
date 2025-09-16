use crate::models::*;
use crate::schema::*;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use reqwest::Client;
use regex::Regex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{Semaphore, RwLock};
use tracing::{error, info, warn, debug};
use url::Url;
use once_cell::sync::Lazy;

/// Global regex patterns for performance
static HANDLE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"/products/([^/]+)").unwrap());

/// High-performance parallel Shopify product scraper
pub struct ShopifyScraper {
    client: Client,
    semaphore: Arc<Semaphore>,
    timeout: Duration,
    config: ScraperConfig,
    domain_limits: Arc<RwLock<HashMap<String, Instant>>>,
}

impl ShopifyScraper {
    /// Create a new scraper instance with advanced configuration
    pub fn new(input: ScraperInput) -> Result<Self> {
        let config = ScraperConfig {
            input: input.clone(),
            user_agent: "ShopifyLightningScraper/1.0 (Rust)".to_string(),
            rate_limit_delay: input.caching.rate_limit_per_domain_ms, // Use input setting
            max_redirects: 5,
        };

        let mut client_builder = Client::builder()
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .danger_accept_invalid_certs(true) // For Docker environments with SSL issues
            .danger_accept_invalid_hostnames(true);

        // Only set timeout if it's greater than 0
        if input.timeout_seconds > 0 {
            client_builder = client_builder.timeout(Duration::from_secs(input.timeout_seconds));
        }

        // Apply performance optimizations
        if input.performance.enable_connection_pooling {
            client_builder = client_builder.pool_max_idle_per_host(20);
        }

        if input.performance.enable_compression {
            client_builder = client_builder.gzip(true).brotli(true);
        }

        let client = client_builder.build()?;
        let semaphore = Arc::new(Semaphore::new(input.max_concurrent));

        Ok(Self {
            client,
            semaphore,
            timeout: Duration::from_secs(input.timeout_seconds),
            config,
            domain_limits: Arc::new(RwLock::new(HashMap::new())),
        })
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

    /// Check if domain appears to be a Shopify store
    fn is_shopify_store(&self, domain: &str) -> bool {
        let domain_lower = domain.to_lowercase();
        domain_lower.contains("myshopify.com") || 
        domain_lower.contains("shopify") ||
        domain_lower.contains("cdn.shopify.com")
    }

    /// Apply product filters
    fn apply_filters(&self, product: &ShopifyProduct) -> bool {
        let filters = &self.config.input.filters;

        // Price filters
        if let Some(min_price) = filters.min_price {
            if product.price < min_price {
                return false;
            }
        }
        if let Some(max_price) = filters.max_price {
            if product.price > max_price {
                return false;
            }
        }

        // Currency filter
        if let Some(ref currency) = filters.currency {
            if product.currency != *currency {
                return false;
            }
        }

        // Vendor filter
        if !filters.vendors.is_empty() {
            if !filters.vendors.contains(&product.vendor) {
                return false;
            }
        }

        // Product type filter
        if !filters.product_types.is_empty() {
            if !filters.product_types.contains(&product.product_type) {
                return false;
            }
        }

        // Tag filters
        if !filters.tags_any.is_empty() {
            if !filters.tags_any.iter().any(|tag| product.tags.contains(tag)) {
                return false;
            }
        }

        // Availability filter
        if let Some(availability) = filters.availability {
            if product.availability != availability {
                return false;
            }
        }

        // Search query filter
        if let Some(ref query) = filters.search_query {
            let search_text = format!("{} {}", product.title, product.description).to_lowercase();
            if !search_text.contains(&query.to_lowercase()) {
                return false;
            }
        }

        true
    }

    /// Fetch product data from Shopify product.json endpoint with retries
    async fn fetch_product_data(&self, domain: &str, product_handle: &str) -> Result<Option<RawShopifyProduct>> {
        let _permit = self.semaphore.acquire().await?;
        
        // Rate limiting per domain
        if self.should_rate_limit(domain).await {
            tokio::time::sleep(Duration::from_millis(self.config.rate_limit_delay)).await;
        }
        
        let url = format!("{}/products/{}.json", domain, product_handle);

        let mut attempts = 0;
        let max_attempts = if self.config.input.performance.enable_retries {
            self.config.input.performance.max_retries + 1
        } else {
            1
        };

        while attempts < max_attempts {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    match response.status() {
                        reqwest::StatusCode::OK => {
                            let api_response: ShopifyApiResponse = response.json().await?;
                            let product = api_response.product;
                            
                            return Ok(Some(product));
                        }
                        reqwest::StatusCode::NOT_FOUND => {
                            warn!("Product not found: {}", product_handle);
                            return Ok(None);
                        }
                        reqwest::StatusCode::TOO_MANY_REQUESTS => {
                            warn!("Rate limited for {}", product_handle);
                            if attempts < max_attempts - 1 {
                                tokio::time::sleep(Duration::from_millis(
                                    1000 * (attempts + 1) as u64
                                )).await;
                                attempts += 1;
                                continue;
                            }
                            return Err(anyhow!("Rate limited"));
                        }
                        status => {
                            error!("HTTP {} for {}", status, url);
                            return Err(anyhow!("HTTP error: {}", status));
                        }
                    }
                }
                Err(e) => {
                    error!("Error fetching {}: {}", product_handle, e);
                    if attempts < max_attempts - 1 {
                        tokio::time::sleep(Duration::from_millis(
                            1000 * (attempts + 1) as u64
                        )).await;
                        attempts += 1;
                        continue;
                    }
                    return Err(anyhow!("Request failed: {}", e));
                }
            }
        }

        Err(anyhow!("Max retries exceeded"))
    }

    /// Transform raw Shopify product data to canonical format
    fn transform_to_canonical(&self, raw_product: RawShopifyProduct, domain: &str) -> Result<ShopifyProduct> {
        // Extract basic product information
        let id = raw_product.id.to_string();
        let title = raw_product.title;
        let description = raw_product.body_html
            .unwrap_or_default()
            .replace("<p>", "")
            .replace("</p>", "")
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");

        // Handle pricing from first variant
        let (price, currency) = if let Some(variant) = raw_product.variants.first() {
            let price: f64 = variant.price.parse().unwrap_or(0.0);
            (price, "USD".to_string()) // Shopify typically uses USD
        } else {
            (0.0, "USD".to_string())
        };

        // Extract images with enhanced data
        let images: Vec<ProductImage> = raw_product.images
            .into_iter()
            .map(|img| ProductImage {
                src: img.src,
                alt: img.alt,
                width: img.width,
                height: img.height,
                position: img.position,
            })
            .collect();

        // Transform variants to canonical format
        let variants: Vec<ProductVariant> = raw_product.variants
            .into_iter()
            .map(|variant| ProductVariant {
                id: variant.id.to_string(),
                title: variant.title,
                price: variant.price.parse().unwrap_or(0.0),
                sku: variant.sku,
                inventory_quantity: variant.inventory_quantity.unwrap_or(0),
                available: variant.available.unwrap_or(false),
                weight: variant.weight.unwrap_or(0.0),
                weight_unit: variant.weight_unit.unwrap_or("kg".to_string()),
                option1: variant.option1,
                option2: variant.option2,
                option3: variant.option3,
                barcode: variant.barcode,
                compare_at_price: variant.compare_at_price.and_then(|p| p.parse().ok()),
                fulfillment_service: variant.fulfillment_service,
                inventory_management: variant.inventory_management,
                inventory_policy: variant.inventory_policy,
                requires_shipping: variant.requires_shipping,
                taxable: variant.taxable,
                tax_code: variant.tax_code,
            })
            .collect();

        // Parse tags
        let tags: Vec<String> = raw_product.tags
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();

        // Extract custom fields from metafields
        let custom_fields = if self.config.input.extraction.include_custom_fields {
            raw_product.metafields.map(|metafields| {
                metafields.into_iter()
                    .map(|mf| (format!("{}.{}", mf.namespace, mf.key), mf.value))
                    .collect()
            })
        } else {
            None
        };

        // Create canonical product
        Ok(ShopifyProduct {
            id,
            title,
            description,
            price,
            currency,
            availability: raw_product.available.unwrap_or(false),
            vendor: raw_product.vendor,
            product_type: raw_product.product_type,
            tags,
            images,
            variants,
            created_at: raw_product.created_at,
            updated_at: raw_product.updated_at,
            handle: raw_product.handle.clone(),
            url: format!("{}/products/{}", domain, raw_product.handle),
            seo_data: None, // Can be populated from additional API calls
            analytics_data: None, // Can be populated from analytics APIs
            related_products: None, // Can be populated from recommendations API
            reviews: None, // Can be populated from reviews API
            collections: None, // Can be populated from collections API
            custom_fields,
            shipping_info: None, // Can be populated from shipping API
            return_policy: None, // Can be populated from policy API
            warranty: None, // Can be populated from warranty API
            // Language-specific fields (can be populated from additional API calls)
            title_de: None,
            title_fr: None,
            title_es: None,
            description_de: None,
            description_fr: None,
            description_es: None,
        })
    }

    /// Scrape a single product and return canonical format
    pub async fn scrape_product(&self, domain: &str, product_handle: &str) -> Result<Option<ShopifyProduct>> {
        let domain = self.normalize_domain(domain)?;

        // Skip Shopify store detection warning for custom domains

        match self.fetch_product_data(&domain, product_handle).await? {
            Some(raw_data) => {
                let canonical = self.transform_to_canonical(raw_data, &domain)?;
                if self.apply_filters(&canonical) {
                    Ok(Some(canonical))
                } else {
                    debug!("Product {} filtered out", product_handle);
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Scrape multiple products in parallel
    pub async fn scrape_multiple_products(&self, domain: &str, product_handles: Vec<String>) -> Result<Vec<ShopifyProduct>> {
        let domain = self.normalize_domain(domain)?;

        info!("Scraping {} products from {}", product_handles.len(), domain);
        let start_time = std::time::Instant::now();

        // Create tasks for parallel execution
        let tasks: Vec<_> = product_handles
            .into_iter()
            .map(|handle| {
                let scraper = self.clone();
                let domain = domain.clone();
                tokio::spawn(async move {
                    scraper.scrape_product(&domain, &handle).await
                })
            })
            .collect();

        // Execute all tasks in parallel
        let results = join_all(tasks).await;

        // Filter out None results and errors
        let mut products = Vec::new();
        for result in results {
            match result {
                Ok(Ok(Some(product))) => products.push(product),
                Ok(Ok(None)) => {} // Product not found or filtered out, skip
                Ok(Err(e)) => error!("Scraping error: {}", e),
                Err(e) => error!("Task error: {}", e),
            }
        }

        let elapsed = start_time.elapsed();
        info!("Scraped {} products in {:.3} seconds", products.len(), elapsed.as_secs_f64());

        Ok(products)
    }

    /// Discover product handles from Shopify store with parallel pagination
    pub async fn discover_products(&self, domain: &str, max_products: usize) -> Result<Vec<String>> {
        let domain = self.normalize_domain(domain)?;

        // Build URLs with pagination parameters
        let limit = self.config.input.pagination.limit;
        let page = self.config.input.pagination.page;
        let enable_pagination = self.config.input.pagination.enable_pagination;
        
        info!("Pagination config: limit={}, page={}, enable_pagination={}", limit, page, enable_pagination);
        
        let urls_to_try = if enable_pagination {
            vec![
                format!("{}/products.json?limit={}&page={}", domain, limit, page),
            ]
        } else {
            vec![
                format!("{}/products.json?limit={}", domain, limit),
            ]
        };

        info!("URLs to try for {}: {:?}", domain, urls_to_try);
        for url in urls_to_try {
            info!("Trying URL: {}", url);
            match self.client.get(&url).send().await {
                Ok(response) if response.status() == reqwest::StatusCode::OK => {
                    info!("Successfully fetched {}", url);
                    info!("URL contains .json: {}", url.contains(".json"));
                    if url.contains(".json") {
                        // First, discover total number of pages by checking first page
                        let response_text = match response.text().await {
                            Ok(text) => {
                                info!("Got response text for {}, length: {}", url, text.len());
                                text
                            }
                            Err(e) => {
                                warn!("Failed to get response text for {}: {}", url, e);
                                continue;
                            }
                        };
                        
                        let first_page_data = match serde_json::from_str::<serde_json::Value>(&response_text) {
                            Ok(data) => {
                                info!("Parsed JSON response for {}", url);
                                data
                            }
                            Err(e) => {
                                warn!("Failed to parse JSON for {}: {}", url, e);
                                warn!("Response text preview: {}", &response_text[..std::cmp::min(200, response_text.len())]);
                                continue; // Try next URL
                            }
                        };
                        if let Some(first_products) = first_page_data.get("products").and_then(|p| p.as_array()) {
                            info!("Found products array with {} products", first_products.len());
                            if first_products.is_empty() {
                                warn!("Products array is empty for {}, trying next URL", url);
                                continue; // Try next URL
                            }
                            
                            // Determine total pages by checking if we got a full page
                            let mut total_pages = 1;
                            if first_products.len() == limit { // Full page, likely more pages exist
                                info!("First page has {} products, checking for more pages...", limit);
                                total_pages = self.discover_total_pages(&domain, &url, limit).await.unwrap_or(1);
                            } else {
                                info!("First page has {} products, likely the only page", first_products.len());
                            }
                            
                            info!("Discovered {} total pages for {}", total_pages, domain);
                            
                            // Create parallel tasks for all pages
                            let mut page_tasks = Vec::new();
                            let max_pages = if self.config.input.pagination.max_pages > 0 {
                                std::cmp::min(total_pages, self.config.input.pagination.max_pages)
                            } else {
                                total_pages
                            };
                            
                            for page_num in 1..=max_pages {
                                let client = self.client.clone();
                                let page_url = format!("{}/products.json?page={}&limit={}", domain, page_num, limit);
                                
                                page_tasks.push(tokio::spawn(async move {
                                    match client.get(&page_url).send().await {
                                        Ok(response) if response.status() == reqwest::StatusCode::OK => {
                                            match response.json::<serde_json::Value>().await {
                                                Ok(data) => {
                                                    if let Some(products) = data.get("products").and_then(|p| p.as_array()) {
                                                        let handles: Vec<String> = products
                                                            .iter()
                                                            .filter_map(|p| p.get("handle").and_then(|h| h.as_str()))
                                                            .map(|s| s.to_string())
                                                            .collect();
                                                        debug!("Page {} returned {} products", page_num, handles.len());
                                                        Ok(handles)
                                                    } else {
                                                        debug!("Page {} has no products array", page_num);
                                                        Ok(Vec::new())
                                                    }
                                                }
                                                Err(e) => {
                                                    warn!("Failed to parse JSON for page {}: {}", page_num, e);
                                                    Err(e)
                                                }
                                            }
                                        }
                                        Ok(response) => {
                                            debug!("Page {} returned status: {}", page_num, response.status());
                                            Ok(Vec::new()) // Non-200 status
                                        }
                                        Err(e) => {
                                            warn!("Failed to fetch page {}: {}", page_num, e);
                                            Err(e.into())
                                        }
                                    }
                                }));
                            }
                            
                            // Collect results from all parallel tasks
                            let mut all_handles = Vec::new();
                            for task in page_tasks {
                                match task.await {
                                    Ok(Ok(handles)) => {
                                        all_handles.extend(handles);
                                    }
                                    Ok(Err(e)) => {
                                        warn!("Error fetching page: {}", e);
                                    }
                                    Err(e) => {
                                        warn!("Task error: {}", e);
                                    }
                                }
                            }
                            
                            // Remove duplicates and limit if needed
                            all_handles.sort();
                            all_handles.dedup();
                            
                            if !all_handles.is_empty() {
                                let final_count = if max_products > 0 && all_handles.len() > max_products {
                                    all_handles.truncate(max_products);
                                    max_products
                                } else {
                                    all_handles.len()
                                };
                                
                                info!("Discovered {} products from {} (parallel pagination, {} pages)", 
                                      final_count, domain, max_pages);
                                return Ok(all_handles);
                            }
                        } else {
                            warn!("No products array found in JSON response for {}", url);
                        }
                    } else if url.contains(".xml") {
                        let content = response.text().await?;
                        let handles: Vec<String> = HANDLE_PATTERN
                            .captures_iter(&content)
                            .filter_map(|cap| cap.get(1))
                            .map(|m| m.as_str().to_string())
                            .collect();
                        if !handles.is_empty() {
                            info!("Discovered {} products from {} (sitemap)", handles.len(), domain);
                            return Ok(handles);
                        }
                    }
                }
                Ok(response) => {
                    warn!("Non-200 response for {}: {}", url, response.status());
                    continue; // Try next URL
                }
                Err(e) => {
                    warn!("Failed to fetch {}: {}", url, e);
                    warn!("Error details: {:?}", e);
                    continue;
                }
            }
        }

        warn!("Could not discover products automatically");
        Ok(vec![])
    }

    /// Discover total number of pages by sequential checking
    async fn discover_total_pages(&self, domain: &str, base_url: &str, limit: usize) -> Result<usize> {
        // Sequential approach: keep checking pages until we get an empty response
        let mut page = 2; // Start from page 2 since we already checked page 1
        let mut last_valid_page = 1;
        
        loop {
            let test_url = format!("{}/products.json?page={}&limit={}", domain, page, limit);
            
            match self.client.get(&test_url).send().await {
                Ok(response) if response.status() == reqwest::StatusCode::OK => {
                    match response.json::<serde_json::Value>().await {
                        Ok(data) => {
                            if let Some(products) = data.get("products").and_then(|p| p.as_array()) {
                                if products.is_empty() {
                                    // Empty page means we've reached the end
                                    break;
                                } else {
                                    last_valid_page = page;
                                    page += 1;
                                    
                                    // Safety check to prevent infinite loops
                                    if page > 1000 {
                                        warn!("Reached maximum page limit (1000), stopping pagination");
                                        break;
                                    }
                                }
                            } else {
                                // No products array means we've reached the end
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse JSON for page {}: {}", page, e);
                            break;
                        }
                    }
                }
                Ok(response) => {
                    // Non-200 status means we've reached the end
                    debug!("Page {} returned status: {}", page, response.status());
                    break;
                }
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                    break;
                }
            }
        }
        
        info!("Found {} total pages for {}", last_valid_page, domain);
        Ok(last_valid_page)
    }
}

impl Clone for ShopifyScraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
            timeout: self.timeout,
            config: self.config.clone(),
            domain_limits: self.domain_limits.clone(),
        }
    }
}