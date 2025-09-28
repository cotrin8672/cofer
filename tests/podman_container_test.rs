use anyhow::Result;
use cofer::podman::PodmanClient;
use std::collections::HashMap;

/// Test container creation
#[tokio::test]
#[ignore] // Requires Podman
async fn test_create_container() -> Result<()> {
    let client = PodmanClient::new().await?;

    // Ensure we have the alpine image
    client.ensure_image("docker.io/library/alpine:latest").await?;

    let container_name = format!("test-container-{}", uuid::Uuid::new_v4());
    let mut env_vars = HashMap::new();
    env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

    let container_id = client.create_container(
        &container_name,
        "alpine:latest",
        "/tmp/test-project",
        "/workdir",
        env_vars,
    ).await?;

    assert!(!container_id.is_empty());

    // Clean up
    client.remove_container(&container_id, true).await?;

    Ok(())
}

/// Test full container lifecycle
#[tokio::test]
#[ignore] // Requires Podman
async fn test_container_lifecycle() -> Result<()> {
    let client = PodmanClient::new().await?;

    // Ensure image exists
    client.ensure_image("docker.io/library/alpine:latest").await?;

    let container_name = format!("test-lifecycle-{}", uuid::Uuid::new_v4());

    // Create container
    let container_id = client.create_container(
        &container_name,
        "alpine:latest",
        "/tmp",
        "/workdir",
        HashMap::new(),
    ).await?;

    // Start container
    client.start_container(&container_id).await?;

    // Execute command
    let exec_result = client.exec_command(
        &container_id,
        vec!["echo".to_string(), "hello world".to_string()],
        None,
    ).await?;

    assert_eq!(exec_result.stdout.trim(), "hello world");
    assert_eq!(exec_result.exit_code, Some(0));

    // Get logs
    let (stdout, stderr) = client.get_logs(&container_id, Some("10".to_string())).await?;
    assert!(stdout.is_empty() || stderr.is_empty()); // Logs should be minimal

    // Stop container
    client.stop_container(&container_id, Some(5)).await?;

    // Remove container
    client.remove_container(&container_id, true).await?;

    Ok(())
}

/// Test container listing
#[tokio::test]
#[ignore] // Requires Podman
async fn test_list_containers() -> Result<()> {
    let client = PodmanClient::new().await?;

    // Create a test container
    client.ensure_image("docker.io/library/alpine:latest").await?;

    let container_name = format!("test-list-{}", uuid::Uuid::new_v4());
    let container_id = client.create_container(
        &container_name,
        "alpine:latest",
        "/tmp",
        "/workdir",
        HashMap::new(),
    ).await?;

    // List all containers (including stopped)
    let containers = client.list_containers(true).await?;

    // Should find our container
    let found = containers.iter().any(|c| {
        c.id.as_ref()
            .map(|id| id.starts_with(&container_id[..12]))
            .unwrap_or(false)
    });

    assert!(found, "Created container should be in the list");

    // Clean up
    client.remove_container(&container_id, true).await?;

    Ok(())
}

/// Test command execution with environment variables
#[tokio::test]
#[ignore] // Requires Podman
async fn test_exec_with_env() -> Result<()> {
    let client = PodmanClient::new().await?;

    // Create and start a container
    client.ensure_image("docker.io/library/alpine:latest").await?;

    let container_name = format!("test-exec-env-{}", uuid::Uuid::new_v4());
    let container_id = client.create_container(
        &container_name,
        "alpine:latest",
        "/tmp",
        "/workdir",
        HashMap::new(),
    ).await?;

    client.start_container(&container_id).await?;

    // Execute command with environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

    let exec_result = client.exec_command(
        &container_id,
        vec!["sh".to_string(), "-c".to_string(), "echo $CUSTOM_VAR".to_string()],
        Some(env_vars),
    ).await?;

    assert_eq!(exec_result.stdout.trim(), "custom_value");
    assert_eq!(exec_result.exit_code, Some(0));

    // Clean up
    client.stop_container(&container_id, Some(5)).await?;
    client.remove_container(&container_id, true).await?;

    Ok(())
}

/// Test bind mount functionality
#[tokio::test]
#[ignore] // Requires Podman and filesystem access
async fn test_bind_mount() -> Result<()> {
    use std::fs;
    use tempfile::tempdir;

    let client = PodmanClient::new().await?;

    // Create temporary directory for testing
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello from host")?;

    // Create container with bind mount
    client.ensure_image("docker.io/library/alpine:latest").await?;

    let container_name = format!("test-mount-{}", uuid::Uuid::new_v4());
    let container_id = client.create_container(
        &container_name,
        "alpine:latest",
        temp_dir.path().to_str().unwrap(),
        "/workdir",
        HashMap::new(),
    ).await?;

    client.start_container(&container_id).await?;

    // Read the mounted file inside container
    let exec_result = client.exec_command(
        &container_id,
        vec!["cat".to_string(), "/workdir/test.txt".to_string()],
        None,
    ).await?;

    assert_eq!(exec_result.stdout.trim(), "Hello from host");

    // Write from container
    let exec_result = client.exec_command(
        &container_id,
        vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo 'Hello from container' > /workdir/container.txt".to_string(),
        ],
        None,
    ).await?;

    assert_eq!(exec_result.exit_code, Some(0));

    // Clean up
    client.stop_container(&container_id, Some(5)).await?;
    client.remove_container(&container_id, true).await?;

    // Verify file was written to host
    let container_file = temp_dir.path().join("container.txt");
    let content = fs::read_to_string(container_file)?;
    assert_eq!(content.trim(), "Hello from container");

    Ok(())
}

/// Test error handling for non-existent container
#[tokio::test]
#[ignore] // Requires Podman
async fn test_operations_on_nonexistent_container() -> Result<()> {
    let client = PodmanClient::new().await?;

    let fake_id = "nonexistent-container-12345";

    // Start should fail
    let result = client.start_container(fake_id).await;
    assert!(result.is_err());

    // Stop should fail
    let result = client.stop_container(fake_id, None).await;
    assert!(result.is_err());

    // Exec should fail
    let result = client.exec_command(
        fake_id,
        vec!["echo".to_string(), "test".to_string()],
        None,
    ).await;
    assert!(result.is_err());

    // Logs should fail
    let result = client.get_logs(fake_id, None).await;
    assert!(result.is_err());

    Ok(())
}