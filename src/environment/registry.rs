use super::handle::EnvironmentHandle;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Registry for managing active environments
#[derive(Clone)]
pub struct EnvironmentRegistry {
    /// Map of environment ID to handle
    environments: Arc<RwLock<HashMap<String, EnvironmentHandle>>>,
}

impl EnvironmentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            environments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new environment
    pub async fn register(&self, handle: EnvironmentHandle) -> Result<()> {
        let env_id = handle.env_id.clone();

        // Requirement 2.4: Atomically check for duplicates and insert
        // Use write lock for the entire operation to prevent race conditions
        let mut envs = self.environments.write().await;

        // Check if environment already exists
        if envs.contains_key(&env_id) {
            bail!("Environment '{}' already exists", env_id);
        }

        // Register the environment
        debug!("Registering environment: {}", env_id);
        envs.insert(env_id.clone(), handle);

        // Release the lock before logging (drop guard)
        drop(envs);

        info!("Environment '{}' registered successfully", env_id);
        Ok(())
    }

    /// Get an environment by ID
    pub async fn get(&self, env_id: &str) -> Result<EnvironmentHandle> {
        let envs = self.environments.read().await;

        envs.get(env_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env_id))
    }

    /// Update an environment
    pub async fn update(&self, handle: EnvironmentHandle) -> Result<()> {
        let env_id = handle.env_id.clone();
        let mut envs = self.environments.write().await;

        if !envs.contains_key(&env_id) {
            bail!("Environment '{}' not found", env_id);
        }

        debug!("Updating environment: {}", env_id);
        envs.insert(env_id, handle);
        Ok(())
    }

    /// Remove an environment
    pub async fn remove(&self, env_id: &str) -> Result<EnvironmentHandle> {
        let mut envs = self.environments.write().await;

        match envs.remove(env_id) {
            Some(handle) => {
                info!("Environment '{}' removed from registry", env_id);
                Ok(handle)
            }
            None => bail!("Environment '{}' not found", env_id),
        }
    }

    /// List all environment IDs
    pub async fn list(&self) -> Vec<String> {
        let envs = self.environments.read().await;
        envs.keys().cloned().collect()
    }

    /// Get all environments
    pub async fn list_all(&self) -> Vec<EnvironmentHandle> {
        let envs = self.environments.read().await;
        envs.values().cloned().collect()
    }

    /// Clear all environments (for cleanup)
    pub async fn clear(&self) -> Vec<EnvironmentHandle> {
        let mut envs = self.environments.write().await;
        let handles: Vec<EnvironmentHandle> = envs.values().cloned().collect();

        if !handles.is_empty() {
            warn!("Clearing {} environments from registry", handles.len());
            envs.clear();
        }

        handles
    }

    /// Get the count of registered environments
    pub async fn count(&self) -> usize {
        let envs = self.environments.read().await;
        envs.len()
    }
}

impl Default for EnvironmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::EnvironmentStatus;
    use std::path::PathBuf;

    fn create_test_handle(env_id: &str) -> EnvironmentHandle {
        EnvironmentHandle::new(
            env_id,
            format!("container-{}", env_id),
            PathBuf::from("/test/path"),
            "alpine:latest",
        )
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = EnvironmentRegistry::new();
        assert_eq!(registry.count().await, 0);
        assert!(registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_and_get() {
        // Requirement 2.1, 4.1: Register and retrieve environments
        let registry = EnvironmentRegistry::new();
        let handle = create_test_handle("env1");

        // Register
        registry.register(handle.clone()).await.unwrap();
        assert_eq!(registry.count().await, 1);

        // Get
        let retrieved = registry.get("env1").await.unwrap();
        assert_eq!(retrieved.env_id, "env1");
        assert_eq!(retrieved.container_id, handle.container_id);
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        // Requirement 2.4: Prevent duplicate environment IDs
        let registry = EnvironmentRegistry::new();
        let handle1 = create_test_handle("env1");
        let handle2 = create_test_handle("env1");

        // First registration should succeed
        registry.register(handle1).await.unwrap();

        // Second registration with same ID should fail
        let result = registry.register(handle2).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        // Requirement 3.5: Error on nonexistent environment
        let registry = EnvironmentRegistry::new();

        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_update_environment() {
        let registry = EnvironmentRegistry::new();
        let mut handle = create_test_handle("env1");

        // Register
        registry.register(handle.clone()).await.unwrap();

        // Update status
        handle.set_status(EnvironmentStatus::Running);
        registry.update(handle.clone()).await.unwrap();

        // Verify update
        let retrieved = registry.get("env1").await.unwrap();
        assert_eq!(retrieved.status, EnvironmentStatus::Running);
    }

    #[tokio::test]
    async fn test_remove_environment() {
        let registry = EnvironmentRegistry::new();
        let handle = create_test_handle("env1");

        // Register
        registry.register(handle.clone()).await.unwrap();
        assert_eq!(registry.count().await, 1);

        // Remove
        let removed = registry.remove("env1").await.unwrap();
        assert_eq!(removed.env_id, "env1");
        assert_eq!(registry.count().await, 0);

        // Try to get removed environment
        let result = registry.get("env1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_environments() {
        let registry = EnvironmentRegistry::new();

        // Register multiple environments
        for i in 1..=3 {
            let handle = create_test_handle(&format!("env{}", i));
            registry.register(handle).await.unwrap();
        }

        // List IDs
        let ids = registry.list().await;
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"env1".to_string()));
        assert!(ids.contains(&"env2".to_string()));
        assert!(ids.contains(&"env3".to_string()));

        // List all handles
        let handles = registry.list_all().await;
        assert_eq!(handles.len(), 3);
    }

    #[tokio::test]
    async fn test_clear_registry() {
        // Requirement 4.2: Clean up all environments
        let registry = EnvironmentRegistry::new();

        // Register multiple environments
        for i in 1..=3 {
            let handle = create_test_handle(&format!("env{}", i));
            registry.register(handle).await.unwrap();
        }

        assert_eq!(registry.count().await, 3);

        // Clear all
        let cleared = registry.clear().await;
        assert_eq!(cleared.len(), 3);
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        // Requirement 4.1, 5.5: Concurrent access safety
        let registry = Arc::new(EnvironmentRegistry::new());
        let mut handles = vec![];

        // Spawn multiple tasks to register environments concurrently
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let env_handle = create_test_handle(&format!("env{}", i));
                reg.register(env_handle).await
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all were registered
        assert_eq!(registry.count().await, 10);

        // Concurrent reads
        let mut read_handles = vec![];
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                reg.get(&format!("env{}", i)).await
            });
            read_handles.push(handle);
        }

        // Wait and verify all reads succeeded
        for handle in read_handles {
            assert!(handle.await.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn test_no_race_condition_on_duplicate_registration() {
        // Test that duplicate registration is properly prevented even with concurrent attempts
        let registry = Arc::new(EnvironmentRegistry::new());
        let mut handles = vec![];

        // Try to register the same environment ID from multiple tasks
        for _ in 0..10 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let env_handle = create_test_handle("same-env");
                reg.register(env_handle).await
            });
            handles.push(handle);
        }

        // Collect results
        let mut success_count = 0;
        let mut failure_count = 0;

        for handle in handles {
            match handle.await.unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            }
        }

        // Exactly one should succeed, the rest should fail
        assert_eq!(success_count, 1);
        assert_eq!(failure_count, 9);

        // Verify only one environment is registered
        assert_eq!(registry.count().await, 1);
    }
}