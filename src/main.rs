mod git;

use crate::git::init_remote_repository;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "cofer")]
#[command(about = "Container environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// setup remote repository and gitignore
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init_remote_repository(Path::new(".")).await?,
    }

    Ok(())
}
