use std::fs;
use std::io::{self, Write};
use std::path::Path;
use anyhow::Result;

pub struct InitCommand;

impl InitCommand {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, path: &Path) -> Result<()> {
        // Create project directory
        fs::create_dir_all(path)?;

        // Get package name from path, or use current directory name if path is "."
        let package_name = if path.to_string_lossy() == "." {
            let current_dir = std::env::current_dir()?;
            let name = current_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Could not determine package name from current directory"))?;
            name.to_string()
        } else {
            path.file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Could not determine package name from path"))?
                .to_string()
        };

        // Create msc.toml (for package configuration)
        let msc_toml = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
description = "A Roblox package"
authors = ["Your Name <your.email@example.com>"]

[dependencies]
# Format: "username/package-name" = "version"
# Example: "sesocell/profilestore" = "1.0.0"
"#,
            package_name
        );

        fs::write(path.join("msc.toml"), msc_toml)?;

        // Create basic directory structure
        fs::create_dir_all(path.join("src"))?;
        fs::create_dir_all(path.join("test"))?;

        // Create .gitignore
        let gitignore = r#"# MSC
.msc/
.packages/
"#;
        fs::write(path.join(".gitignore"), gitignore)?;

        // Create README.md
        let readme = format!(
            r#"# {}

A Roblox package managed by MSC.

## Installation

"#,
            package_name
        );
        fs::write(path.join("README.md"), readme)?;

        println!("✨ Created new MSC package: {}", path.display());
        println!("📦 Next steps:");
        println!("  cd {}", path.display());
        println!("  # Add your package files to src/");
        println!("  # Edit msc.toml to add dependencies");
        println!("  msc publish    # When ready to publish");

        Ok(())
    }
}

pub fn execute() -> Result<()> {
    println!("Creating new MSC package...");

    // Get package name from current directory
    let current_dir = std::env::current_dir()?;
    let package_name = current_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not determine package name from current directory"))?;

    // Ask for package description
    print!("Description (optional): ");
    io::stdout().flush()?;
    let mut description = String::new();
    io::stdin().read_line(&mut description)?;
    let description = description.trim();

    // Ask for author
    print!("Author: ");
    io::stdout().flush()?;
    let mut author = String::new();
    io::stdin().read_line(&mut author)?;
    let author = author.trim();

    // Create msc.toml content
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
description = "{}"
author = "{}"

[dependencies]
# Format: "username/package-name" = "version"
# Example: "sesocell/profilestore" = "1.0.0"
"#,
        package_name, description, author
    );

    // Write msc.toml file
    fs::write("msc.toml", toml_content)?;

    println!("✓ Created msc.toml");
    println!("✓ MSC package initialized");

    Ok(())
} 