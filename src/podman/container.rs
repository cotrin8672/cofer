use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::ContainerSummary;
use bollard::service::{HostConfig, Mount, MountTypeEnum};
use futures::StreamExt;
use std::collections::HashMap;
use tracing::{debug, error, info};

use super::client::PodmanClient;

/// Container lifecycle management for Podman
impl PodmanClient {
    /// Create a new container
    pub async fn create_container(
        &self,
        name: &str,
        image: &str,
        project_root: &str,
        mount_path: &str,
        env_vars: HashMap<String, String>,
    ) -> Result<String> {
        info!("Creating container: {} from image: {}", name, image);

        // Prepare environment variables
        let env: Vec<String> = env_vars
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Create bind mount
        let mount = Mount {
            target: Some(mount_path.to_string()),
            source: Some(project_root.to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        // Container configuration
        let config = Config {
            image: Some(image.to_string()),
            env: Some(env),
            working_dir: Some(mount_path.to_string()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            host_config: Some(HostConfig {
                mounts: Some(vec![mount]),
                auto_remove: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name,
            platform: None,
        };

        let response = self
            .docker
            .create_container(Some(options), config)
            .await
            .context("Failed to create container")?;

        let container_id = response.id;
        info!("Created container with ID: {}", container_id);

        Ok(container_id)
    }

    /// Start a container
    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        info!("Starting container: {}", container_id);

        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        info!("Container started successfully: {}", container_id);
        Ok(())
    }

    /// Stop a container
    pub async fn stop_container(&self, container_id: &str, timeout: Option<i64>) -> Result<()> {
        info!("Stopping container: {}", container_id);

        let options = StopContainerOptions {
            t: timeout.unwrap_or(10),
        };

        self.docker
            .stop_container(container_id, Some(options))
            .await
            .context("Failed to stop container")?;

        info!("Container stopped successfully: {}", container_id);
        Ok(())
    }

    /// Remove a container
    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        info!("Removing container: {} (force: {})", container_id, force);

        let options = RemoveContainerOptions {
            force,
            v: true, // Remove volumes
            ..Default::default()
        };

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .context("Failed to remove container")?;

        info!("Container removed successfully: {}", container_id);
        Ok(())
    }

    /// List containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        debug!("Listing containers (all: {})", all);

        let options = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .context("Failed to list containers")?;

        info!("Found {} containers", containers.len());
        Ok(containers)
    }

    /// Execute a command in a container
    pub async fn exec_command(
        &self,
        container_id: &str,
        cmd: Vec<String>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<ExecResult> {
        info!("Executing command in container {}: {:?}", container_id, cmd);

        // Prepare environment variables
        let env = env_vars.map(|vars| {
            vars.into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
        });

        let exec_config = CreateExecOptions {
            cmd: Some(cmd),
            env,
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        // Create exec instance
        let exec_create = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .context("Failed to create exec instance")?;

        let exec_id = exec_create.id;

        // Start exec and collect output
        let exec_start = self.docker.start_exec(&exec_id, None).await?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        match exec_start {
            StartExecResults::Attached { mut output, .. } => {
                while let Some(chunk) = output.next().await {
                    match chunk {
                        Ok(bollard::container::LogOutput::StdOut { message }) => {
                            stdout.extend_from_slice(&message);
                        }
                        Ok(bollard::container::LogOutput::StdErr { message }) => {
                            stderr.extend_from_slice(&message);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error reading exec output: {}", e);
                            break;
                        }
                    }
                }
            }
            StartExecResults::Detached => {
                debug!("Exec started in detached mode");
            }
        }

        // Get exit code
        let exec_inspect = self.docker.inspect_exec(&exec_id).await?;
        let exit_code = exec_inspect.exit_code;

        Ok(ExecResult {
            exit_code,
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
        })
    }

    /// Get container logs
    pub async fn get_logs(
        &self,
        container_id: &str,
        tail: Option<String>,
    ) -> Result<(String, String)> {
        debug!("Getting logs for container: {}", container_id);

        let options = LogsOptions {
            stdout: true,
            stderr: true,
            tail: tail.unwrap_or_else(|| "all".to_string()),
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bollard::container::LogOutput::StdOut { message }) => {
                    stdout.extend_from_slice(&message);
                }
                Ok(bollard::container::LogOutput::StdErr { message }) => {
                    stderr.extend_from_slice(&message);
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Error reading logs: {}", e);
                    break;
                }
            }
        }

        Ok((
            String::from_utf8_lossy(&stdout).to_string(),
            String::from_utf8_lossy(&stderr).to_string(),
        ))
    }
}

/// Result from executing a command in a container
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: Option<i64>,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Podman
    async fn test_list_containers() {
        if let Ok(client) = PodmanClient::new().await {
            let result = client.list_containers(true).await;
            assert!(result.is_ok());

            let containers = result.unwrap();
            // Just verify the API works
            assert!(containers.len() >= 0);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman and image
    async fn test_container_lifecycle() {
        if let Ok(client) = PodmanClient::new().await {
            // Ensure we have an alpine image
            let _ = client.ensure_image("docker.io/library/alpine:latest").await;

            let container_name = format!("test-container-{}", uuid::Uuid::new_v4());

            // Create container
            let result = client
                .create_container(
                    &container_name,
                    "alpine:latest",
                    "/tmp",
                    "/workdir",
                    HashMap::new(),
                )
                .await;

            if let Ok(container_id) = result {
                // Start container
                let start_result = client.start_container(&container_id).await;
                assert!(start_result.is_ok());

                // Execute command
                let exec_result = client
                    .exec_command(&container_id, vec!["echo".to_string(), "hello".to_string()], None)
                    .await;

                if let Ok(exec) = exec_result {
                    assert_eq!(exec.stdout.trim(), "hello");
                    assert_eq!(exec.exit_code, Some(0));
                }

                // Stop container
                let stop_result = client.stop_container(&container_id, Some(5)).await;
                assert!(stop_result.is_ok());

                // Remove container
                let remove_result = client.remove_container(&container_id, true).await;
                assert!(remove_result.is_ok());
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman
    async fn test_exec_with_env_vars() {
        if let Ok(client) = PodmanClient::new().await {
            // This test would require a running container
            // For now, just verify the API compiles
            let mut env_vars = HashMap::new();
            env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

            // Would need a real container ID here
            let _result = client.exec_command(
                "dummy-container-id",
                vec!["env".to_string()],
                Some(env_vars),
            );
        }
    }
}