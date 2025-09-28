use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{RwLock, watch};
use tracing::{debug, error, info, warn};

use super::handlers;
use super::types::{McpError, McpRequest, McpResponse};
use crate::environment::EnvironmentRegistry;

/// MCP server that handles JSON-RPC requests over stdio
pub struct McpServer {
    /// Registry of method handlers
    handlers: HashMap<String, Box<dyn handlers::Handler>>,
    /// Shared state for the server
    state: Arc<RwLock<ServerState>>,
}

/// Server state that can be shared across handlers
#[derive(Clone)]
pub struct ServerState {
    /// Environment registry for managing container environments
    pub registry: EnvironmentRegistry,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            registry: EnvironmentRegistry::new(),
        }
    }
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn handlers::Handler>> = HashMap::new();

        // Register core handlers
        handlers.insert("initialize".to_string(), Box::new(handlers::InitializeHandler));
        handlers.insert("create_environment".to_string(), Box::new(handlers::CreateEnvironmentHandler));
        handlers.insert("run_command".to_string(), Box::new(handlers::RunCommandHandler));

        // Register unimplemented handlers
        handlers.insert("watch-commit".to_string(), Box::new(handlers::UnimplementedHandler));
        handlers.insert("note-append".to_string(), Box::new(handlers::UnimplementedHandler));
        handlers.insert("up".to_string(), Box::new(handlers::UnimplementedHandler));
        handlers.insert("down".to_string(), Box::new(handlers::UnimplementedHandler));

        Self {
            handlers,
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }

    /// Run the server, listening on stdio with LSP-style transport
    pub async fn run(&mut self, mut shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        info!("MCP server starting on stdio with Content-Length headers");

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut stdout = stdout;

        loop {
            // Check for shutdown signal
            if *shutdown_rx.borrow() {
                info!("Shutdown signal received");
                break;
            }

            // Try to read the next message with a timeout
            let message = tokio::select! {
                result = self.read_message(&mut reader) => {
                    match result {
                        Ok(Some(msg)) => msg,
                        Ok(None) => {
                            // EOF reached
                            info!("EOF reached, shutting down");
                            break;
                        },
                        Err(e) => {
                            error!("Error reading message: {}", e);
                            // Try to continue if possible
                            continue;
                        }
                    }
                },
                _ = shutdown_rx.changed() => {
                    info!("Shutdown signal received during read");
                    break;
                }
            };

            debug!("Received message: {}", message);

            // Parse and handle the request
            let response = self.handle_request(&message).await;

            // Send response with Content-Length header
            let response_str = serde_json::to_string(&response)?;
            let header = format!("Content-Length: {}\r\n\r\n", response_str.len());

            debug!("Sending response with header: {} bytes", response_str.len());

            stdout.write_all(header.as_bytes()).await?;
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.flush().await?;
        }

        info!("MCP server shutting down");
        Ok(())
    }

    /// Read a message with Content-Length header
    async fn read_message<R>(&self, reader: &mut BufReader<R>) -> Result<Option<String>>
    where
        R: AsyncReadExt + Unpin,
    {
        let mut header_line = String::new();

        // Read until we find Content-Length header
        loop {
            header_line.clear();

            // Read a line
            let bytes_read = reader.read_line(&mut header_line).await?;
            if bytes_read == 0 {
                // EOF
                return Ok(None);
            }

            // Check for Content-Length header
            if header_line.starts_with("Content-Length: ") {
                break;
            }

            // Skip other headers if any
            if header_line.trim().is_empty() {
                // Empty line without Content-Length is unexpected
                continue;
            }
        }

        // Parse content length
        let len_str = header_line
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?
            .trim();

        let content_length: usize = len_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid Content-Length value: {}", e))?;

        // Read until empty line (end of headers)
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            }
        }

        // Read exact content length
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).await?;

        Ok(Some(String::from_utf8(content)?))
    }

    /// Handle a single JSON-RPC request
    async fn handle_request(&self, input: &str) -> McpResponse {
        // Parse the JSON
        let request = match serde_json::from_str::<McpRequest>(input) {
            Ok(req) => req,
            Err(e) => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError::invalid_request(format!("Invalid JSON: {}", e))),
                };
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError::invalid_request("Invalid JSON-RPC version")),
            };
        }

        // Check if method exists
        let handler = match self.handlers.get(&request.method) {
            Some(h) => h,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError::method_not_found(&request.method)),
                };
            }
        };

        // Execute the handler
        match handler.handle(&request, &self.state).await {
            Ok(result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(error) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
            },
        }
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down MCP server");

        // Clean up environments
        // Clone the registry to avoid holding the lock across await
        let registry = {
            let state = self.state.read().await;
            state.registry.clone()
        };

        let environments = registry.clear().await;

        if !environments.is_empty() {
            warn!("Cleaning up {} active environments", environments.len());
            // TODO: Actually stop/remove containers via Podman
            for env in environments {
                debug!("Cleanup required for environment: {}", env.env_id);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_server_creation() {
        let server = McpServer::new();
        assert!(!server.handlers.is_empty());
        assert!(server.handlers.contains_key("initialize"));
        assert!(server.handlers.contains_key("create_environment"));
        assert!(server.handlers.contains_key("run_command"));
    }

    #[tokio::test]
    async fn test_invalid_json_handling() {
        let server = McpServer::new();
        let response = server.handle_request("invalid json").await;

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[tokio::test]
    async fn test_valid_request_structure() {
        let server = McpServer::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let response = server.handle_request(&request.to_string()).await;
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let server = McpServer::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unknown_method",
            "params": {}
        });

        let response = server.handle_request(&request.to_string()).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_unimplemented_method() {
        let server = McpServer::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "watch-commit",
            "params": {}
        });

        let response = server.handle_request(&request.to_string()).await;
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32601); // Unimplemented methods return MethodNotFound
        assert!(error.message.to_lowercase().contains("unimplemented"));
    }

    #[tokio::test]
    async fn test_read_message_with_content_length() {
        let server = McpServer::new();

        // Create a test message with Content-Length header
        let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let message = format!("Content-Length: {}\r\n\r\n{}", json_content.len(), json_content);

        let mut reader = BufReader::new(message.as_bytes());
        let result = server.read_message(&mut reader).await.unwrap();

        assert_eq!(result, Some(json_content.to_string()));
    }

    #[tokio::test]
    async fn test_read_message_eof() {
        let server = McpServer::new();
        let mut reader = BufReader::new("".as_bytes());
        let result = server.read_message(&mut reader).await.unwrap();
        assert_eq!(result, None);
    }
}