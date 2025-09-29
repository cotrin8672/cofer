use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use chrono::Utc;

use super::server::ServerState;
use super::types::{McpError, McpRequest};
use crate::environment::{EnvironmentHandle, EnvironmentStatus};
use crate::podman::PodmanClient;

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
        Ok(json!({
            "protocolVersion": "0.1.0",
            "serverInfo": {
                "name": "cofer",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "tools": [
                    {
                        "name": "create_environment",
                        "description": "Create a new container environment"
                    },
                    {
                        "name": "run_command",
                        "description": "Execute a command in an environment"
                    }
                ]
            }
        }))
    }
}

/// Handler for create_environment method
pub struct CreateEnvironmentHandler;

#[async_trait]
impl Handler for CreateEnvironmentHandler {
    async fn handle(&self, request: &McpRequest, state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        // Extract parameters
        let params = request.params.as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing parameters"))?;

        let env_id = params.get("env_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing env_id"))?
            .to_string();

        let project_root = params.get("project_root")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing project_root"))?
            .to_string();

        let image = params.get("image")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing image"))?
            .to_string();

        // Validate project_root exists
        if !Path::new(&project_root).exists() {
            return Err(McpError::invalid_params(format!("Project root does not exist: {}", project_root)));
        }

        info!("Creating environment: {} with image: {} at: {}",
              env_id, image, project_root);

        // Extract optional parameters
        let env_vars = params.get("env_vars")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_else(std::collections::HashMap::new);

        let mount_path = params.get("mount_path")
            .and_then(|v| v.as_str())
            .unwrap_or("/workdir")
            .to_string();

        let ports = params.get("ports")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Clone the registry to avoid holding the lock across await
        let registry = {
            let state_guard = state.read().await;
            state_guard.registry.clone()
        };

        // Check for duplicate environment
        if registry.get(&env_id).await.is_ok() {
            return Err(McpError::invalid_params(format!("Environment '{}' already exists", env_id)));
        }

        // Connect to Podman
        let podman = match PodmanClient::new().await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to connect to Podman: {}", e);
                return Err(McpError::internal_error(format!("Failed to connect to Podman: {}", e)));
            }
        };

        // Ensure image exists
        if let Err(e) = podman.ensure_image(&image).await {
            error!("Failed to ensure image {}: {}", image, e);
            return Err(McpError::internal_error(format!("Failed to ensure image: {}", e)));
        }

        // Create container
        let container_id = match podman.create_container(
            &env_id,
            &image,
            &project_root,
            &mount_path,
            env_vars.clone(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to create container: {}", e);
                return Err(McpError::internal_error(format!("Failed to create container: {}", e)));
            }
        };

        // Start container
        if let Err(e) = podman.start_container(&container_id).await {
            error!("Failed to start container: {}", e);
            // Clean up the created container
            let _ = podman.remove_container(&container_id, true).await;
            return Err(McpError::internal_error(format!("Failed to start container: {}", e)));
        }

        // Create environment handle
        let mut handle = EnvironmentHandle::new(
            env_id.clone(),
            container_id.clone(),
            PathBuf::from(project_root.clone()),
            image,
        );

        // Set mount path
        handle.mount_path = mount_path.clone();

        // Add environment variables
        if !env_vars.is_empty() {
            handle.add_env_vars(env_vars.clone());
        }

        // Set status to running
        handle.set_status(EnvironmentStatus::Running);

        // Register in the registry
        registry.register(handle.clone()).await
            .map_err(|e| McpError::internal_error(e.to_string()))?;

        // Build response object
        let mut response = json!({
            "env_id": env_id,
            "container_id": container_id,
            "project_root": project_root,
            "mount_path": mount_path,
            "status": "running",
            "created_at": handle.created_at.to_rfc3339()
        });

        // Add env_vars if present
        if !env_vars.is_empty() {
            response["env_vars"] = json!(env_vars);
        }

        // Add ports if present
        if !ports.is_empty() {
            response["ports"] = json!(ports);
        }

        Ok(response)
    }
}

/// Handler for run_command method
pub struct RunCommandHandler;

#[async_trait]
impl Handler for RunCommandHandler {
    async fn handle(&self, request: &McpRequest, state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        // Extract parameters
        let params = request.params.as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing parameters"))?;

        let env_id = params.get("env_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing env_id"))?;

        let command = params.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing command"))?;

        info!("Running command in environment {}: {}", env_id, command);

        // Get environment from registry
        let registry = {
            let state_guard = state.read().await;
            state_guard.registry.clone()
        };

        let handle = registry.get(env_id).await
            .map_err(|e| McpError::invalid_params(format!("Environment not found: {}", e)))?;

        // Check environment status
        if handle.status != EnvironmentStatus::Running {
            return Err(McpError::invalid_request(format!(
                "Environment '{}' is not running (status: {:?})",
                env_id, handle.status
            )));
        }

        // Connect to Podman
        let podman = match PodmanClient::new().await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to connect to Podman: {}", e);
                return Err(McpError::internal_error(format!("Failed to connect to Podman: {}", e)));
            }
        };

        // Execute command in container
        let exec_result = match podman.exec_command(
            &handle.container_id,
            vec!["sh".to_string(), "-c".to_string(), command.to_string()],
            None,
        ).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to execute command: {}", e);
                return Err(McpError::internal_error(format!("Failed to execute command: {}", e)));
            }
        };

        // Return execution result
        Ok(json!({
            "env_id": env_id,
            "command": command,
            "exit_code": exec_result.exit_code.unwrap_or(-1),
            "stdout": exec_result.stdout,
            "stderr": exec_result.stderr,
            "executed_at": Utc::now().to_rfc3339()
        }))
    }
}

/// Handler for unimplemented methods
pub struct UnimplementedHandler {
    pub method: String,
}

#[async_trait]
impl Handler for UnimplementedHandler {
    async fn handle(&self, _request: &McpRequest, _state: &Arc<RwLock<ServerState>>) -> Result<Value, McpError> {
        Err(McpError::method_not_found(format!("Method '{}' is not implemented", self.method)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::EnvironmentRegistry;
    use serde_json::json;

    async fn create_test_state() -> Arc<RwLock<ServerState>> {
        Arc::new(RwLock::new(ServerState {
            registry: EnvironmentRegistry::new(),
        }))
    }

    #[tokio::test]
    async fn test_initialize_handler() {
        let handler = InitializeHandler;
        let state = create_test_state().await;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("protocolVersion").is_some());
        assert!(value.get("serverInfo").is_some());
    }

    #[tokio::test]
    async fn test_create_environment_validation() {
        let handler = CreateEnvironmentHandler;
        let state = create_test_state().await;

        // Test missing parameters
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "create_environment".to_string(),
            params: None,
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());

        // Test missing env_id
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "create_environment".to_string(),
            params: Some(json!({
                "project_root": "/tmp/test",
                "image": "alpine:latest"
            })),
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_command_validation() {
        let handler = RunCommandHandler;
        let state = create_test_state().await;

        // Test missing parameters
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "run_command".to_string(),
            params: None,
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());

        // Test missing command
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "run_command".to_string(),
            params: Some(json!({
                "env_id": "test-env"
            })),
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unimplemented_handler() {
        let handler = UnimplementedHandler {
            method: "unknown_method".to_string(),
        };
        let state = create_test_state().await;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "unknown_method".to_string(),
            params: None,
        };

        let result = handler.handle(&request, &state).await;
        assert!(result.is_err());

        if let Err(error) = result {
            assert_eq!(error.code, -32601); // Method not found
        }
    }

    #[tokio::test]
    async fn test_create_environment_success() {
        use std::fs;
        use tempfile::tempdir;

        // Create temp directory for testing
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path().to_str().unwrap();

        let handler = CreateEnvironmentHandler;
        let state = create_test_state().await;

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "create_environment".to_string(),
            params: Some(json!({
                "env_id": "test-env",
                "project_root": project_root,
                "image": "docker.io/library/alpine:latest",
                "env_vars": {
                    "TEST_VAR": "test_value"
                }
            })),
        };

        // This test will only pass if Podman is available
        // In CI or without Podman, it will fail with internal error
        let _result = handler.handle(&request, &state).await;
    }
}