use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Canonical product data structure following the four immutable rules:
/// 1. Language: English
/// 2. Case: camelCase  
/// 3. Specificity: Clear but Concise
/// 4. Singularity & Plurality: Strict Rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopifyProduct {
    pub id: String,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub availability: bool,
    pub vendor: String,
    pub product_type: String,
    pub tags: Vec<String>,
    pub images: Vec<ProductImage>,
    pub variants: Vec<ProductVariant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub handle: String,
    pub url: String,
    
    // Advanced data fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seo_data: Option<SeoData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics_data: Option<AnalyticsData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_products: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviews: Option<ReviewsData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collections: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_info: Option<ShippingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty: Option<String>,
    
    // Language-specific fields follow suffixing convention
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_de: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_fr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_es: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_de: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_fr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_es: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductImage {
    pub src: String,
    pub alt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub position: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub title: String,
    pub price: f64,
    pub sku: String,
    pub inventory_quantity: i32,
    pub available: bool,
    pub weight: f64,
    pub weight_unit: String,
    pub option1: Option<String>,
    pub option2: Option<String>,
    pub option3: Option<String>,
    pub barcode: Option<String>,
    pub compare_at_price: Option<f64>,
    pub fulfillment_service: Option<String>,
    pub inventory_management: Option<String>,
    pub inventory_policy: Option<String>,
    pub requires_shipping: Option<bool>,
    pub taxable: Option<bool>,
    pub tax_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoData {
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub canonical_url: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub views: Option<u64>,
    pub conversions: Option<u64>,
    pub conversion_rate: Option<f64>,
    pub revenue: Option<f64>,
    pub profit_margin: Option<f64>,
    pub inventory_turnover: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewsData {
    pub average_rating: Option<f64>,
    pub total_reviews: Option<u32>,
    pub rating_distribution: Option<HashMap<String, u32>>,
    pub recent_reviews: Option<Vec<Review>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub author: String,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingInfo {
    pub free_shipping_threshold: Option<f64>,
    pub shipping_methods: Vec<ShippingMethod>,
    pub estimated_delivery: Option<String>,
    pub international_shipping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingMethod {
    pub name: String,
    pub price: f64,
    pub currency: String,
    pub delivery_time: String,
    pub free_shipping: bool,
}

/// Raw Shopify API response structure
#[derive(Debug, Deserialize)]
pub struct ShopifyApiResponse {
    pub product: RawShopifyProduct,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawShopifyProduct {
    pub id: u64,
    pub title: String,
    pub body_html: Option<String>,
    pub vendor: String,
    pub product_type: String,
    pub tags: String,
    pub available: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub handle: String,
    pub variants: Vec<RawProductVariant>,
    pub images: Vec<RawProductImage>,
    pub options: Option<Vec<RawProductOption>>,
    pub metafields: Option<Vec<RawMetafield>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProductVariant {
    pub id: u64,
    pub title: String,
    pub price: String,
    pub sku: String,
    pub inventory_quantity: Option<i32>,
    pub available: Option<bool>,
    pub weight: Option<f64>,
    pub weight_unit: Option<String>,
    pub option1: Option<String>,
    pub option2: Option<String>,
    pub option3: Option<String>,
    pub barcode: Option<String>,
    pub compare_at_price: Option<String>,
    pub fulfillment_service: Option<String>,
    pub inventory_management: Option<String>,
    pub inventory_policy: Option<String>,
    pub requires_shipping: Option<bool>,
    pub taxable: Option<bool>,
    pub tax_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProductImage {
    pub src: String,
    pub alt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub position: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProductOption {
    pub id: u64,
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMetafield {
    pub id: u64,
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub value_type: String,
}