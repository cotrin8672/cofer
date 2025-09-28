use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::server::ServerState;
use super::types::{McpError, McpRequest};
use crate::environment::{EnvironmentHandle, EnvironmentStatus};

/// Trait for handling MCP methods
#[async_trait]
pub trait Handler: Send + Sync {
    /// Handle a request and return a result or error
    async fn handle(&self, request: &McpRequest, state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError>;
}

/// Handler for the initialize method
pub struct InitializeHandler;

#[async_trait]
impl Handler for InitializeHandler {
    async fn handle(&self, _request: &McpRequest, _state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        info!("Handling initialize request");

        // Return server capabilities
        Ok(json!({
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {
                    "create_environment": {
                        "description": "Create a new container environment"
                    },
                    "run_command": {
                        "description": "Run a command in an environment"
                    }
                }
            },
            "serverInfo": {
                "name": "cofer",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }
}

/// Handler for create_environment method
pub struct CreateEnvironmentHandler;

#[async_trait]
impl Handler for CreateEnvironmentHandler {
    async fn handle(&self, request: &McpRequest, state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        info!("Handling create_environment request");

        // Extract parameters
        let params = request.params.as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing parameters"))?;

        let project_root = params.get("project_root")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing or invalid project_root"))?;

        let env_id = params.get("env_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing or invalid env_id"))?;

        let image = params.get("image")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing or invalid image"))?;

        // Parse optional environment variables
        let env_vars = params.get("env_vars")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_else(std::collections::HashMap::new);

        // Create environment handle
        let mut handle = EnvironmentHandle::new(
            env_id,
            format!("mock-container-{}", env_id), // TODO: Get real container ID from Podman
            PathBuf::from(project_root),
            image,
        );

        // Add environment variables
        if !env_vars.is_empty() {
            handle.add_env_vars(env_vars);
        }

        // Register in the registry
        // Clone the registry to avoid holding the lock across await
        let registry = {
            let state_guard = state.read().await;
            state_guard.registry.clone()
        };

        registry.register(handle.clone()).await
            .map_err(|e| McpError::invalid_request(e.to_string()))?;

        // TODO: Actually create the container with Podman
        // For now, update status to running
        handle.set_status(EnvironmentStatus::Running);

        // Update the handle in registry
        registry.update(handle.clone()).await
            .map_err(|e| McpError::internal_error(e.to_string()))?;

        // Return the environment details
        Ok(serde_json::to_value(&handle).unwrap_or_else(|_| {
            json!({
                "env_id": env_id,
                "container_id": handle.container_id,
                "project_root": handle.project_root.to_string_lossy(),
                "mount_path": handle.mount_path,
                "status": "running"
            })
        }))
    }
}

/// Handler for run_command method
pub struct RunCommandHandler;

#[async_trait]
impl Handler for RunCommandHandler {
    async fn handle(&self, request: &McpRequest, state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        info!("Handling run_command request");

        // Extract parameters
        let params = request.params.as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing parameters"))?;

        let env_id = params.get("env_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing or invalid env_id"))?;

        let _cmd = params.get("cmd")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_params("Missing or invalid cmd"))?;

        let _timeout_ms = params.get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(120000); // Default 2 minutes

        // Check if environment exists and is running
        // Clone the registry to avoid holding the lock across await
        let registry = {
            let state_guard = state.read().await;
            state_guard.registry.clone()
        };

        let handle = registry.get(env_id).await
            .map_err(|_| McpError::invalid_params(
                format!("Environment '{}' not found", env_id)
            ))?;

        if !handle.is_running() {
            return Err(McpError::invalid_params(
                format!("Environment '{}' is not running (status: {:?})", env_id, handle.status)
            ));
        }

        // TODO: Actually execute the command with Podman
        // For now, return a mock response

        Ok(json!({
            "exit_code": 0,
            "stdout_tail": "Mock output from command",
            "stderr_tail": "",
            "execution_time_ms": 100,
            "timed_out": false
        }))
    }
}

/// Handler for unimplemented methods
pub struct UnimplementedHandler;

#[async_trait]
impl Handler for UnimplementedHandler {
    async fn handle(&self, request: &McpRequest, _state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        info!("Unimplemented method called: {}", request.method);

        Err(McpError {
            code: -32601,
            message: format!("Method '{}' is unimplemented", request.method),
            data: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize_handler() {
        let handler = InitializeHandler;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };
        let state = Arc::new(RwLock::new(ServerState::default()));

        let result = handler.handle(&request, &state).await.unwrap();
        assert!(result.get("protocolVersion").is_some());
        assert!(result.get("capabilities").is_some());
        assert!(result.get("serverInfo").is_some());
    }

    #[tokio::test]
    async fn test_create_environment_validation() {
        let handler = CreateEnvironmentHandler;
        let state = Arc::new(RwLock::new(ServerState::default()));

        // Test missing parameters
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "create_environment".to_string(),
            params: None,
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32602);

        // Test missing project_root
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "create_environment".to_string(),
            params: Some(json!({
                "env_id": "test",
                "image": "alpine"
            })),
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32602);
    }

    #[tokio::test]
    async fn test_create_environment_success() {
        let handler = CreateEnvironmentHandler;
        let state = Arc::new(RwLock::new(ServerState::default()));

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "create_environment".to_string(),
            params: Some(json!({
                "project_root": "/test",
                "env_id": "test-env",
                "image": "alpine:latest"
            })),
        };

        let result = handler.handle(&request, &state).await.unwrap();
        assert_eq!(result["env_id"], "test-env");
        assert_eq!(result["mount_path"], "/workdir");

        // Verify environment was stored
        let registry = {
            let state_guard = state.read().await;
            state_guard.registry.clone()
        };
        let stored = registry.get("test-env").await;
        assert!(stored.is_ok());
        assert_eq!(stored.unwrap().env_id, "test-env");
    }

    #[tokio::test]
    async fn test_run_command_validation() {
        let handler = RunCommandHandler;
        let state = Arc::new(RwLock::new(ServerState::default()));

        // Test environment not found
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "run_command".to_string(),
            params: Some(json!({
                "env_id": "nonexistent",
                "cmd": ["echo", "hello"]
            })),
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32602);
    }

    #[tokio::test]
    async fn test_unimplemented_handler() {
        let handler = UnimplementedHandler;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "watch-commit".to_string(),
            params: Some(json!({})),
        };
        let state = Arc::new(RwLock::new(ServerState::default()));

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("unimplemented"));
    }
}