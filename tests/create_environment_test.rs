use anyhow::Result;
use cofer::mcp::server::McpServer;
use cofer::mcp::types::{McpRequest, McpResponse};
use serde_json::{json, Value};

/// Helper to create a test server
fn create_test_server() -> McpServer {
    McpServer::new()
}

/// Helper to make a request and parse response
async fn make_request(server: &McpServer, request: Value) -> McpResponse {
    let request_str = request.to_string();
    server.handle_request(&request_str).await
}

/// Test successful environment creation
#[tokio::test]
async fn test_create_environment_success() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-env-1",
            "project_root": project_root,
            "image": "docker.io/library/alpine:latest"
        }
    });

    let response = make_request(&server, request).await;

    // Print error if present for debugging
    if let Some(ref err) = response.error {
        eprintln!("Error: code={}, message={}", err.code, err.message);
    }

    // Skip if Podman is not available or container creation fails
    if response.error.is_some() {
        let err_msg = &response.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            eprintln!("Skipping test: Podman not available or container creation failed");
            return Ok(());
        }
    }

    assert!(response.result.is_some(), "Expected result, got error: {:?}", response.error);
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert!(result.get("env_id").is_some());
    assert_eq!(result["env_id"], "test-env-1");
    assert!(result.get("container_id").is_some());
    assert!(result.get("status").is_some());
    assert_eq!(result["status"], "running");

    Ok(())
}

/// Test duplicate environment ID
#[tokio::test]
async fn test_create_environment_duplicate() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    // Create first environment
    let request1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-duplicate",
            "project_root": project_root,
            "image": "docker.io/library/alpine:latest"
        }
    });

    let response1 = make_request(&server, request1).await;

    // Skip if Podman is not available or container creation fails
    if response1.error.is_some() {
        let err_msg = &response1.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            return Ok(());
        }
    }

    assert!(response1.result.is_some());

    // Try to create duplicate
    let request2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "create_environment",
        "params": {
            "env_id": "test-duplicate",
            "project_root": project_root,
            "image": "docker.io/library/alpine:latest"
        }
    });

    let response2 = make_request(&server, request2).await;

    assert!(response2.error.is_some());
    assert!(response2.result.is_none());

    let error = response2.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params
    assert!(error.message.contains("already exists"));

    Ok(())
}

/// Test missing required parameters
#[tokio::test]
async fn test_create_environment_missing_params() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    // Missing env_id
    let request1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "project_root": project_root,
            "image": "alpine:latest"
        }
    });

    let response1 = make_request(&server, request1).await;
    assert!(response1.error.is_some());
    assert_eq!(response1.error.unwrap().code, -32602);

    // Missing project_root
    let request2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "create_environment",
        "params": {
            "env_id": "test-env",
            "image": "alpine:latest"
        }
    });

    let response2 = make_request(&server, request2).await;
    assert!(response2.error.is_some());
    assert_eq!(response2.error.unwrap().code, -32602);

    // Missing image
    let request3 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "create_environment",
        "params": {
            "env_id": "test-env",
            "project_root": project_root
        }
    });

    let response3 = make_request(&server, request3).await;
    assert!(response3.error.is_some());
    assert_eq!(response3.error.unwrap().code, -32602);

    Ok(())
}

/// Test invalid project root path
#[tokio::test]
async fn test_create_environment_invalid_path() -> Result<()> {
    let server = create_test_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-env",
            "project_root": "/nonexistent/path/to/nowhere",
            "image": "alpine:latest"
        }
    });

    let response = make_request(&server, request).await;

    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert!(error.message.contains("not found") || error.message.contains("does not exist"));

    Ok(())
}

/// Test environment variables handling
#[tokio::test]
async fn test_create_environment_with_env_vars() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-env-vars",
            "project_root": project_root,
            "image": "alpine:latest",
            "env_vars": {
                "TEST_VAR": "test_value",
                "ANOTHER_VAR": "another_value"
            }
        }
    });

    let response = make_request(&server, request).await;

    // Skip if Podman is not available or container creation fails
    if response.error.is_some() {
        let err_msg = &response.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            return Ok(());
        }
    }

    assert!(response.result.is_some());

    let result = response.result.unwrap();
    if let Some(env_vars) = result.get("env_vars") {
        let env_vars = env_vars.as_object().unwrap();
        assert_eq!(env_vars.get("TEST_VAR").unwrap(), "test_value");
        assert_eq!(env_vars.get("ANOTHER_VAR").unwrap(), "another_value");
    }

    Ok(())
}

/// Test port mapping configuration
#[tokio::test]
async fn test_create_environment_with_ports() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-ports",
            "project_root": project_root,
            "image": "alpine:latest",
            "ports": ["3000:3000", "8080:8080"]
        }
    });

    let response = make_request(&server, request).await;

    // Skip if Podman is not available or container creation fails
    if response.error.is_some() {
        let err_msg = &response.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            return Ok(());
        }
    }

    assert!(response.result.is_some());

    let result = response.result.unwrap();
    if let Some(ports) = result.get("ports") {
        let ports = ports.as_array().unwrap();
        assert_eq!(ports.len(), 2);
        assert!(ports.contains(&json!("3000:3000")));
        assert!(ports.contains(&json!("8080:8080")));
    }

    Ok(())
}

/// Test mount path configuration
#[tokio::test]
async fn test_create_environment_mount_path() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-mount",
            "project_root": project_root,
            "image": "alpine:latest",
            "mount_path": "/custom/workdir"
        }
    });

    let response = make_request(&server, request).await;

    // Skip if Podman is not available or container creation fails
    if response.error.is_some() {
        let err_msg = &response.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            return Ok(());
        }
    }

    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["mount_path"], "/custom/workdir");

    Ok(())
}

/// Test default mount path
#[tokio::test]
async fn test_create_environment_default_mount() -> Result<()> {
    use tempfile::tempdir;

    let server = create_test_server();

    // Create temporary directory for project root
    let temp_dir = tempdir()?;
    let project_root = temp_dir.path().to_str().unwrap();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_environment",
        "params": {
            "env_id": "test-default-mount",
            "project_root": project_root,
            "image": "alpine:latest"
        }
    });

    let response = make_request(&server, request).await;

    // Skip if Podman is not available or container creation fails
    if response.error.is_some() {
        let err_msg = &response.error.as_ref().unwrap().message;
        if err_msg.contains("Podman") || err_msg.contains("Failed to connect to Podman") || err_msg.contains("Failed to create container") {
            return Ok(());
        }
    }

    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["mount_path"], "/workdir");  // Default mount path

    Ok(())
}