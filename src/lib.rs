pub mod models;
pub mod scraper;
pub mod schema;

pub use scraper::ShopifyScraper;
pub use schema::{ScraperInput, OutputFormat, ProductFilters, ExtractionOptions, PerformanceSettings};
pub use models::{ShopifyProduct, ProductVariant, ProductImage};