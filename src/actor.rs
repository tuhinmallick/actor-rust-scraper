use std::sync::Arc;
use std::env;
use reqwest::Client;
use crate::utils::is_on_apify;
use crate::dataset::DatasetHandle;

#[derive(Clone)]
pub struct Actor {
    pub client: Arc<Client>,
    // TODO: Probably wrap in mutex
    pub dataset_cache: std::collections::HashMap<String, crate::dataset::DatasetHandle>
}

impl Actor {
    /// Creates new Actor handler and initializes client
    pub fn new () -> Actor {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");
        
        Actor {
            client: Arc::new(client),
            dataset_cache: std::collections::HashMap::new(),
        }
    }

    pub async fn open_dataset(&mut self, dataset_name_or_id: Option<&str>, force_cloud: bool)
        -> Result<DatasetHandle, Box<dyn std::error::Error + Send + Sync>> {
        if force_cloud && !env::var("APIFY_TOKEN").is_ok() {
            panic!("Cannot open cloud dataset without a token! Add APIFY_TOKEN env var!")
        }

        // TODO: Fix this remove/insert to clone
        if let Some(dataset) = self.dataset_cache.remove(dataset_name_or_id.unwrap_or("default")) {
            self.dataset_cache.insert(dataset.id.clone(), dataset.clone());
            return Ok(dataset);
        }

        let is_default = dataset_name_or_id.is_none();

        // println!("is_default {}", is_default);

        let dataset;
        if is_on_apify() || force_cloud {
            if is_default {
                let dataset_id = env::var("APIFY_DEFAULT_DATASET_ID")
                    .map_err(|_| "APIFY_DEFAULT_DATASET_ID environment variable not set")?;
                dataset = DatasetHandle {
                    id: dataset_id,
                    name: "default".to_string(),
                    is_on_cloud: true,
                    client: Arc::clone(&self.client),
                }
            } else {
                // Create cloud dataset using Apify API
                let name = dataset_name_or_id.unwrap_or("default");
                let token = env::var("APIFY_TOKEN")
                    .map_err(|_| "APIFY_TOKEN environment variable not set")?;
                let url = "https://api.apify.com/v2/acts/apify~dataset/run-sync";
                
                let payload = serde_json::json!({
                    "name": name
                });
                
                let response = self.client
                    .post(url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await?;
                
                if !response.status().is_success() {
                    return Err(format!("Failed to create cloud dataset: {}", response.status()).into());
                }
                
                let dataset_info: serde_json::Value = response.json().await?;
                let dataset_id = dataset_info.get("data")
                    .and_then(|d| d.get("id"))
                    .and_then(|id| id.as_str())
                    .ok_or("Failed to get dataset ID from response")?;
                
                dataset = DatasetHandle {
                    id: dataset_id.to_string(),
                    name: name.to_string(),
                    is_on_cloud: true,
                    client: Arc::clone(&self.client),
                }
            }
        } else {
            let name = dataset_name_or_id.unwrap_or("default");
            // Will return error if the dir already exists
            // TODO: Handle properly
            std::fs::create_dir_all(format!("apify_storage/datasets/{}", name))?;
            dataset = DatasetHandle {
                id: name.to_string(),
                name: name.to_string(),
                is_on_cloud: false,
                client: Arc::clone(&self.client),
            }
        }
        self.dataset_cache.insert(dataset.id.clone(), dataset.clone());
        Ok(dataset)
    }

    /// Pushes data to default dataset (initializes default DatasetHandle)
    pub async fn push_data<T: serde::Serialize> (&mut self, data: &[T]) 
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dataset_handle = self.open_dataset(None, false).await?;
        dataset_handle.push_data(data).await?;
        Ok(())
    }
}