use anyhow::Result;
use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info};

mod environment;
mod mcp;
mod service;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing - output to stderr to avoid interfering with MCP protocol on stdout
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cofer=debug,rmcp=info".into()),
        )
        .init();

    info!("Starting Cofer MCP Server v{}", env!("CARGO_PKG_VERSION"));

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create MCP server
    let mut server = mcp::McpServer::new();

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run(shutdown_rx).await {
            error!("MCP server error: {}", e);
        }
        if let Err(e) = server.shutdown().await {
            error!("Server shutdown error: {}", e);
        }
    });

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal (Ctrl+C)");
            // Send shutdown signal to server
            let _ = shutdown_tx.send(true);
        }
        Err(e) => {
            error!("Failed to listen for shutdown signal: {}", e);
        }
    }

    // Wait for server to finish with timeout
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        server_handle
    ).await {
        Ok(Ok(_)) => info!("Server task completed successfully"),
        Ok(Err(e)) => error!("Server task failed: {}", e),
        Err(_) => {
            error!("Server shutdown timed out after 5 seconds");
        }
    }

    info!("Cofer MCP Server stopped");
    Ok(())
}