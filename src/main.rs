use anyhow::Result;

mod service;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cofer=debug,rmcp=info".into()),
        )
        .init();

    tracing::info!("Starting Cofer MCP Server");

    // For now, just print a message - will implement MCP later
    println!("Cofer MCP Server v{}", env!("CARGO_PKG_VERSION"));
    println!("Ready to accept MCP connections via stdio");

    // TODO: Implement actual MCP server with rmcp
    // The rmcp API needs more investigation to use correctly

    Ok(())
}