use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use std::fs;

const GITHUB_API_URL: &str = "https://api.github.com/repos/mosaicgames/msc-index/contents";

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub dependencies: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubContent {
    name: String,
    #[serde(rename = "type")]
    content_type: String,
    download_url: Option<String>,
    path: String,
}

pub struct Registry {
    client: Client,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_package(&self, username: &str, package_name: &str) -> Result<Package> {
        // msc.toml dosyasını GitHub'dan çek
        let config_url = format!("https://raw.githubusercontent.com/mosaicgames/msc-index/main/{}/{}/msc.toml", username, package_name);
        println!("Fetching package info from: {}", config_url);
        
        let response = self.client
            .get(&config_url)
            .header("User-Agent", "msc-package-manager")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Package not found: @{}/{} (Status: {})", username, package_name, response.status());
        }

        // Raw içeriği doğrudan TOML olarak parse et
        let toml_content = response.text().await?;
        let package: Package = toml::from_str(&toml_content)?;
        Ok(package)
    }

    pub async fn download_package(&self, username: &str, package_name: &str, dest: &Path) -> Result<u64> {
        Box::pin(async move {
            let mut total_bytes = 0u64;
            let api_url = format!("{}/{}/{}", GITHUB_API_URL, username, package_name);
            
            let response = self.client
                .get(&api_url)
                .header("User-Agent", "msc-package-manager")
                .send()
                .await?;

            if !response.status().is_success() {
                anyhow::bail!("Failed to get package contents: {}", response.status());
            }

            let contents: Vec<GithubContent> = response.json().await?;

            for content in contents {
                let target_path = dest.join(&content.name);
                
                if content.content_type == "dir" {
                    fs::create_dir_all(&target_path)?;
                    total_bytes += self.download_package(username, &content.path, &target_path).await?;
                } else if content.content_type == "file" {
                    if let Some(download_url) = content.download_url {
                        let file_response = self.client
                            .get(&download_url)
                            .send()
                            .await?;
                        
                        let file_content = file_response.bytes().await?;
                        total_bytes += file_content.len() as u64;
                        fs::write(&target_path, file_content)?;
                    }
                }
            }

            Ok(total_bytes)
        }).await
    }
} 