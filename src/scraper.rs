use crate::models::*;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use reqwest::Client;
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use url::Url;

/// High-performance parallel Shopify product scraper
pub struct ShopifyScraper {
    client: Client,
    semaphore: Arc<Semaphore>,
    timeout: Duration,
}

impl ShopifyScraper {
    /// Create a new scraper instance
    pub fn new(max_concurrent: usize, timeout_secs: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;
        
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        
        Ok(Self {
            client,
            semaphore,
            timeout: Duration::from_secs(timeout_secs),
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
    
    /// Check if domain appears to be a Shopify store
    fn is_shopify_store(&self, domain: &str) -> bool {
        let domain_lower = domain.to_lowercase();
        domain_lower.contains("myshopify.com") || 
        domain_lower.contains("shopify") ||
        domain_lower.contains("cdn.shopify.com")
    }
    
    /// Fetch product data from Shopify product.json endpoint
    async fn fetch_product_data(&self, domain: &str, product_handle: &str) -> Result<Option<RawShopifyProduct>> {
        let _permit = self.semaphore.acquire().await?;
        
        let url = format!("{}/products/{}.json", domain, product_handle);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                match response.status() {
                    reqwest::StatusCode::OK => {
                        let api_response: ShopifyApiResponse = response.json().await?;
                        Ok(Some(api_response.product))
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        warn!("Product not found: {}", product_handle);
                        Ok(None)
                    }
                    status => {
                        error!("HTTP {} for {}", status, url);
                        Err(anyhow!("HTTP error: {}", status))
                    }
                }
            }
            Err(e) => {
                error!("Error fetching {}: {}", product_handle, e);
                Err(anyhow!("Request failed: {}", e))
            }
        }
    }
    
    /// Transform raw Shopify product data to canonical format
    fn transform_to_canonical(&self, raw_product: RawShopifyProduct) -> ShopifyProduct {
        // Extract basic product information
        let id = raw_product.id.to_string();
        let title = raw_product.title;
        let description = raw_product.body_html
            .unwrap_or_default()
            .replace("<p>", "")
            .replace("</p>", "");
        
        // Handle pricing from first variant
        let (price, currency) = if let Some(variant) = raw_product.variants.first() {
            let price: f64 = variant.price.parse().unwrap_or(0.0);
            (price, "USD".to_string()) // Shopify typically uses USD
        } else {
            (0.0, "USD".to_string())
        };
        
        // Extract images
        let images: Vec<String> = raw_product.images
            .into_iter()
            .map(|img| img.src)
            .collect();
        
        // Transform variants to canonical format
        let variants: Vec<ProductVariant> = raw_product.variants
            .into_iter()
            .map(|variant| ProductVariant {
                id: variant.id.to_string(),
                title: variant.title,
                price: variant.price.parse().unwrap_or(0.0),
                sku: variant.sku,
                inventory_quantity: variant.inventory_quantity,
                available: variant.available,
                weight: variant.weight,
                weight_unit: variant.weight_unit,
            })
            .collect();
        
        // Parse tags
        let tags: Vec<String> = raw_product.tags
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();
        
        // Create canonical product
        ShopifyProduct {
            id,
            title,
            description,
            price,
            currency,
            availability: raw_product.available,
            vendor: raw_product.vendor,
            product_type: raw_product.product_type,
            tags,
            images,
            variants,
            created_at: raw_product.created_at,
            updated_at: raw_product.updated_at,
            handle: raw_product.handle,
            // Language-specific fields (can be populated from additional API calls)
            title_de: None,
            title_fr: None,
            title_es: None,
            description_de: None,
            description_fr: None,
            description_es: None,
        }
    }
    
    /// Scrape a single product and return canonical format
    pub async fn scrape_product(&self, domain: &str, product_handle: &str) -> Result<Option<ShopifyProduct>> {
        let domain = self.normalize_domain(domain)?;
        
        if !self.is_shopify_store(&domain) {
            warn!("Domain {} may not be a Shopify store", domain);
        }
        
        match self.fetch_product_data(&domain, product_handle).await? {
            Some(raw_data) => Ok(Some(self.transform_to_canonical(raw_data))),
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
                Ok(Ok(None)) => {} // Product not found, skip
                Ok(Err(e)) => error!("Scraping error: {}", e),
                Err(e) => error!("Task error: {}", e),
            }
        }
        
        let elapsed = start_time.elapsed();
        info!("Scraped {} products in {:.3} seconds", products.len(), elapsed.as_secs_f64());
        
        Ok(products)
    }
    
    /// Discover product handles from Shopify store
    pub async fn discover_products(&self, domain: &str, max_products: usize) -> Result<Vec<String>> {
        let domain = self.normalize_domain(domain)?;
        
        // Try to get products from collections or sitemap
        let urls_to_try = vec![
            format!("{}/collections/all/products.json", domain),
            format!("{}/products.json", domain),
            format!("{}/sitemap_products_1.xml", domain),
        ];
        
        for url in urls_to_try {
            match self.client.get(&url).send().await {
                Ok(response) if response.status() == reqwest::StatusCode::OK => {
                    if url.ends_with(".json") {
                        let data: serde_json::Value = response.json().await?;
                        if let Some(products) = data.get("products").and_then(|p| p.as_array()) {
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
                    } else if url.ends_with(".xml") {
                        let content = response.text().await?;
                        let re = Regex::new(r"/products/([^"]+)")?;
                        let handles: Vec<String> = re
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
                Ok(_) => continue, // Try next URL
                Err(e) => {
                    warn!("Failed to fetch {}: {}", url, e);
                    continue;
                }
            }
        }
        
        warn!("Could not discover products automatically");
        Ok(vec![])
    }
}

impl Clone for ShopifyScraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
            timeout: self.timeout,
        }
    }
}