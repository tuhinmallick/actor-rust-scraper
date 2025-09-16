use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub images: Vec<String>,
    pub variants: Vec<ProductVariant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub handle: String,
    
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
pub struct ProductVariant {
    pub id: String,
    pub title: String,
    pub price: f64,
    pub sku: String,
    pub inventory_quantity: i32,
    pub available: bool,
    pub weight: f64,
    pub weight_unit: String,
}

/// Raw Shopify API response structure
#[derive(Debug, Deserialize)]
pub struct ShopifyApiResponse {
    pub product: RawShopifyProduct,
}

#[derive(Debug, Deserialize)]
pub struct RawShopifyProduct {
    pub id: u64,
    pub title: String,
    pub body_html: Option<String>,
    pub vendor: String,
    pub product_type: String,
    pub tags: String,
    pub available: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub handle: String,
    pub variants: Vec<RawProductVariant>,
    pub images: Vec<RawProductImage>,
}

#[derive(Debug, Deserialize)]
pub struct RawProductVariant {
    pub id: u64,
    pub title: String,
    pub price: String,
    pub sku: String,
    pub inventory_quantity: i32,
    pub available: bool,
    pub weight: f64,
    pub weight_unit: String,
}

#[derive(Debug, Deserialize)]
pub struct RawProductImage {
    pub src: String,
}