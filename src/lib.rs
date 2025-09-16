pub mod models;
pub mod scraper;
pub mod output;

pub use scraper::ShopifyScraper;
pub use output::{format_output, OutputFormat};
pub use models::{ShopifyProduct, ProductVariant};