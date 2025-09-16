use crate::models::ShopifyProduct;
use anyhow::Result;
use serde_json;
use clap::ValueEnum;

/// Output format options
#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Csv,
}

/// Format products according to specified output format
pub fn format_output(products: &[ShopifyProduct], output_format: &OutputFormat) -> Result<String> {
    match output_format {
        OutputFormat::Json => {
            Ok(serde_json::to_string_pretty(products)?)
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(vec![]);
            
            for product in products {
                wtr.serialize(product)?;
            }
            
            Ok(String::from_utf8(wtr.into_inner()?)?)
        }
    }
}