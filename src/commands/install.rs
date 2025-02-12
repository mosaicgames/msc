use std::fs;
use std::path::Path;
use std::time::Instant;
use serde::Deserialize;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle, HumanBytes};
use crate::registry::Registry;
use crate::types::PackageConfig;

pub struct InstallCommand;

impl InstallCommand {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, package_spec: &str) -> Result<()> {
        Box::pin(async move {
            let start_time = Instant::now();

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg} {prefix:.cyan}")?
            );

            let (username, package_name) = if package_spec.starts_with('@') {
                let without_at = &package_spec[1..];
                match without_at.split_once('/') {
                    Some((u, p)) => (u, p),
                    None => anyhow::bail!("Invalid package format. Use @username/package-name or username/package-name")
                }
            } else {
                match package_spec.split_once('/') {
                    Some((u, p)) => (u, p),
                    None => anyhow::bail!("Invalid package format. Use @username/package-name or username/package-name")
                }
            };

            pb.set_message(format!("Installing @{}/{}", username, package_name));

            let packages_dir = Path::new("packages").join(format!("@{}/{}", username, package_name));
            fs::create_dir_all(&packages_dir)?;

            let registry = Registry::new();
            let total_bytes = registry.download_package(username, package_name, &packages_dir).await?;

            update_msc_toml(username, package_name, "1.0.0")?;

            let duration = start_time.elapsed();
            let speed = if duration.as_secs_f64() > 0.0 {
                total_bytes as f64 / duration.as_secs_f64()
            } else {
                0.0
            };

            pb.finish_with_message(format!(
                "✓ Installed @{}/{} ({} in {:.2?}, {}/s)", 
                username, package_name,
                HumanBytes(total_bytes),
                duration,
                HumanBytes(speed as u64)
            ));

            Ok(())
        }).await
    }
}

fn update_msc_toml(username: &str, package_name: &str, version: &str) -> Result<()> {
    let toml_path = Path::new("msc.toml");
    let mut config: PackageConfig = if toml_path.exists() {
        let content = fs::read_to_string(toml_path)?;
        toml::from_str(&content)?
    } else {
        PackageConfig {
            package: None,
            dependencies: std::collections::HashMap::new(),
        }
    };

    let dep_name = format!("{}/{}", username, package_name);
    config.dependencies.insert(dep_name, version.to_string());

    let toml_content = toml::to_string(&config)?;
    fs::write(toml_path, toml_content)?;

    Ok(())
}

pub async fn install_from_msc_toml() -> Result<()> {
    let toml_content = fs::read_to_string("msc.toml")?;
    let config: PackageConfig = toml::from_str(&toml_content)?;

    if !config.dependencies.is_empty() {
        let cmd = InstallCommand::new();
        for (name, _version) in config.dependencies {
            cmd.execute(&name).await?;
        }
    }

    Ok(())
}

pub async fn execute(package_name: Option<&str>) -> Result<()> {
    if !Path::new("msc.toml").exists() {
        anyhow::bail!("msc.toml dosyası bulunamadı. Önce 'msc init' komutunu çalıştırın.");
    }

    let toml_content = fs::read_to_string("msc.toml")
        .map_err(|e| anyhow::anyhow!("msc.toml okunamadı: {}", e))?;
    let package_config: PackageConfig = toml::from_str(&toml_content)
        .map_err(|e| anyhow::anyhow!("msc.toml ayrıştırılamadı: {}", e))?;

    match package_name {
        Some(name) => {
            let start = Instant::now();
            install_package(name).await?;
            let duration = start.elapsed();
            println!("Paket başarıyla yüklendi: {} (Süre: {:.2?})", name, duration);
        },
        None => {
            if package_config.dependencies.is_empty() {
                println!("msc.toml'da hiç paket tanımlanmamış.");
                return Ok(());
            }

            println!("Tüm paketler yükleniyor...");
            for (name, _) in package_config.dependencies {
                let start = Instant::now();
                install_package(&name).await?;
                let duration = start.elapsed();
                println!("Paket yüklendi: {} (Süre: {:.2?})", name, duration);
            }
            println!("Tüm paketler başarıyla yüklendi!");
        }
    }
    Ok(())
}

async fn install_package(package_name: &str) -> Result<()> {
    let start = Instant::now();
    let parts: Vec<&str> = package_name.trim_start_matches('@').split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Paket adı '@kullanici/paket' formatında olmalı");
    }
    
    let (username, package) = (parts[0], parts[1]);
    let client = reqwest::Client::new();
    let package_dir = Path::new("packages").join(format!("@{}/{}", username, package));
    fs::create_dir_all(&package_dir)?;

    download_subdirectories(&client, username, package, &package_dir).await?;
    update_package_files(package_name, username, package).await?;
    
    let duration = start.elapsed();
    println!("Paket başarıyla yüklendi: {} (Süre: {:.2?})", package_name, duration);
    Ok(())
}

async fn download_subdirectories(client: &reqwest::Client, username: &str, package: &str, package_dir: &Path) -> Result<u64> {
    Box::pin(async move {
        let mut total_bytes = 0u64;
        let api_url = format!("https://api.github.com/repos/mosaicgames/msc-index/contents/{}/{}", username, package);
        println!("Downloading directory contents: {}", api_url);
        
        let response = client.get(&api_url)
            .header("User-Agent", "msc-package-manager")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Could not connect to GitHub API: {}", e))?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to get directory contents: {}", response.status());
        }

        #[derive(Deserialize)]
        struct GithubContent {
            name: String,
            #[serde(rename = "type")]
            content_type: String,
            download_url: Option<String>,
            path: String,
            size: Option<u64>,
        }

        let contents: Vec<GithubContent> = response.json().await?;

        for content in contents {
            let target_path = package_dir.join(&content.name);
            
            if content.content_type == "dir" {
                println!("Found directory: {}", content.name);
                fs::create_dir_all(&target_path)?;
                total_bytes += download_subdirectories(client, username, &content.path, &target_path).await?;
            } else if content.content_type == "file" {
                if let Some(download_url) = content.download_url {
                    println!("Downloading file: {}", content.name);
                    let file_response = client.get(&download_url)
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

async fn update_package_files(_package_name: &str, _username: &str, _package: &str) -> Result<()> {
    // TODO: Implement package file update logic
    Ok(())
}
