use rand::Rng;
use std::sync::Arc;
use reqwest::Client;
use std::env;

// Handle for both local and cloud datasets. 
// There are some fields that are useless but this is simpler now.
#[derive(Clone)]
pub struct DatasetHandle {
    pub id: String,
    pub name: String,
    pub is_on_cloud: bool,
    pub client: Arc<Client>, // A reference to the actor's client
}

impl DatasetHandle {
    pub async fn push_data<T: serde::Serialize> (&self, data: &[T]) 
        -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_on_cloud {
            // For cloud datasets, use Apify API directly
            let token = env::var("APIFY_TOKEN")?;
            let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", self.id, token);
            
            let json_data = serde_json::to_string(data)?;
            let response = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(json_data)
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(format!("Failed to push data to cloud dataset: {}", response.status()).into());
            }
        } else {
            // For local datasets, save to files
            for val in data.iter() {
                let json = serde_json::to_string(&val)?;
                let mut rng = rand::thread_rng();
                // TODO: Implement increment instead of random
                let path = format!("apify_storage/datasets/{}/{}.json", self.name, rng.gen::<i32>());
                std::fs::write(path, json)?;
            } 
        }
        Ok(())
    }
}