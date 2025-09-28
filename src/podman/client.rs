use anyhow::{Context, Result};
use bollard::Docker;
use std::time::Duration;
use tracing::{debug, error, info};

use super::diagnostics::{PodmanDiagnostics, PodmanStatus};

/// Podman client for container operations
#[derive(Clone)]
pub struct PodmanClient {
    /// Bollard Docker client (Podman-compatible)
    pub(crate) docker: Docker,
    /// Connection status
    pub(crate) status: PodmanStatus,
}

impl PodmanClient {
    /// Create a new Podman client with automatic connection
    pub async fn new() -> Result<Self> {
        // First, run diagnostics
        let status = PodmanDiagnostics::check_podman_available()
            .context("Failed to check Podman availability")?;

        if !status.available {
            PodmanDiagnostics::diagnose()?;
        }

        // Connect based on detected socket
        let docker = Self::connect_with_socket(status.socket_path.as_deref()).await?;

        info!("Podman client connected successfully");

        Ok(Self { docker, status })
    }

    /// Connect using a specific socket path or auto-detect
    async fn connect_with_socket(socket_path: Option<&str>) -> Result<Docker> {
        let docker = if let Some(socket) = socket_path {
            debug!("Connecting to Podman at: {}", socket);

            #[cfg(target_os = "windows")]
            {
                if socket.starts_with("npipe://") {
                    Docker::connect_with_named_pipe(socket, 120, bollard::API_DEFAULT_VERSION)
                        .context("Failed to connect to Podman named pipe")?
                } else {
                    Docker::connect_with_socket(socket, 120, bollard::API_DEFAULT_VERSION)
                        .context("Failed to connect to Podman socket")?
                }
            }

            #[cfg(unix)]
            {
                Docker::connect_with_socket(socket, 120, bollard::API_DEFAULT_VERSION)
                    .context("Failed to connect to Podman socket")?
            }
        } else {
            // Try default connection
            debug!("Attempting default Docker/Podman connection");
            Docker::connect_with_defaults()
                .context("Failed to connect to Podman with default settings")?
        };

        // Verify connection with a ping
        Self::verify_connection(&docker).await?;

        Ok(docker)
    }

    /// Verify the connection is working
    async fn verify_connection(docker: &Docker) -> Result<()> {
        let timeout = Duration::from_secs(5);

        debug!("Verifying Podman connection with ping");

        match tokio::time::timeout(timeout, docker.ping()).await {
            Ok(Ok(_)) => {
                info!("Podman connection verified successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Podman ping failed: {}", e);
                Err(anyhow::anyhow!("Failed to ping Podman: {}", e))
            }
            Err(_) => {
                error!("Podman ping timeout after {:?}", timeout);
                Err(anyhow::anyhow!("Podman ping timeout - service may not be running"))
            }
        }
    }

    /// Get version information
    pub async fn version(&self) -> Result<bollard::models::SystemVersion> {
        self.docker
            .version()
            .await
            .context("Failed to get Podman version")
    }

    /// Get system information
    pub async fn info(&self) -> Result<bollard::models::SystemInfo> {
        self.docker
            .info()
            .await
            .context("Failed to get Podman system info")
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.status.service_running
    }

    /// Get the underlying Docker client for advanced operations
    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    /// Create a new client with custom timeout
    pub async fn with_timeout(timeout_secs: u64) -> Result<Self> {
        let status = PodmanDiagnostics::check_podman_available()?;

        if !status.available {
            PodmanDiagnostics::diagnose()?;
        }

        let docker = if let Some(socket) = status.socket_path.as_deref() {
            Docker::connect_with_socket(socket, timeout_secs, bollard::API_DEFAULT_VERSION)?
        } else {
            Docker::connect_with_defaults()?
        };

        Self::verify_connection(&docker).await?;

        Ok(Self { docker, status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Podman to be running
    async fn test_client_connection() {
        let result = PodmanClient::new().await;

        if result.is_ok() {
            let client = result.unwrap();
            assert!(client.is_connected());

            // Try to get version
            let version = client.version().await;
            assert!(version.is_ok());
        } else {
            // If connection fails, ensure we get a helpful error
            match result {
                Err(err) => {
                    let err_str = err.to_string();
                    assert!(
                        err_str.contains("Podman") || err_str.contains("not running"),
                        "Error should be informative: {}",
                        err_str
                    );
                }
                Ok(_) => panic!("Expected error but got success"),
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman
    async fn test_client_info() {
        if let Ok(client) = PodmanClient::new().await {
            let info = client.info().await;
            assert!(info.is_ok());

            let info = info.unwrap();
            assert!(info.id.is_some() || info.name.is_some());
        }
    }

    #[tokio::test]
    async fn test_custom_timeout() {
        let result = PodmanClient::with_timeout(10).await;

        // This test just verifies the API works
        // Actual connection depends on Podman availability
        match result {
            Err(err) => {
                // Should have a proper error message
                assert!(!err.to_string().is_empty());
            }
            Ok(_) => {
                // Connection succeeded, which is also valid
            }
        }
    }
}