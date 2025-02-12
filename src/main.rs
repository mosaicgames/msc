use clap::{Parser, Subcommand};
use std::path::PathBuf;
use msc::commands;

#[derive(Parser)]
#[clap(name = "msc")]
#[clap(about = "Roblox package manager")]
#[clap(author = "MosaicGames")]
#[clap(version = "0.1.0")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new package
    Init {
        #[clap(default_value = ".")]
        path: PathBuf,
    },
    /// Install packages from msc.toml or install a specific package
    Install {
        /// Optional package name with format @author/package
        package: Option<String>,
    },
    Publish {
        #[clap(short, long)]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            commands::init::InitCommand::new().execute(&path)?;
        }
        Commands::Install { package } => {
            if let Some(pkg) = package {
                commands::install::InstallCommand::new().execute(&pkg).await?;
            } else {
                commands::install::install_from_msc_toml().await?;
            }
        }
        Commands::Publish { token } => {
            commands::publish::PublishCommand::new(token).execute().await?;
        }
    }

    Ok(())
}
