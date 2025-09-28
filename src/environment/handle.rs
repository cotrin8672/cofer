use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of an environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentStatus {
    Creating,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

/// Handle to a container environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentHandle {
    /// Unique environment identifier
    pub env_id: String,

    /// Container ID from Podman
    pub container_id: String,

    /// Project root path on host
    pub project_root: PathBuf,

    /// Mount path in container (always /workdir)
    pub mount_path: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Current status
    pub status: EnvironmentStatus,

    /// Container image used
    pub image: String,

    /// Environment variables
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
}

impl EnvironmentHandle {
    /// Create a new environment handle
    pub fn new(
        env_id: impl Into<String>,
        container_id: impl Into<String>,
        project_root: PathBuf,
        image: impl Into<String>,
    ) -> Self {
        Self {
            env_id: env_id.into(),
            container_id: container_id.into(),
            project_root,
            mount_path: "/workdir".to_string(),
            created_at: Utc::now(),
            status: EnvironmentStatus::Creating,
            image: image.into(),
            env_vars: std::collections::HashMap::new(),
        }
    }

    /// Update the status
    pub fn set_status(&mut self, status: EnvironmentStatus) {
        self.status = status;
    }

    /// Add environment variables
    pub fn add_env_vars(&mut self, vars: std::collections::HashMap<String, String>) {
        self.env_vars.extend(vars);
    }

    /// Check if environment is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, EnvironmentStatus::Running)
    }

    /// Check if environment is in error state
    pub fn is_error(&self) -> bool {
        matches!(self.status, EnvironmentStatus::Error(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_environment_handle_creation() {
        // Requirement 2.1: EnvironmentHandle should store all required fields
        let handle = EnvironmentHandle::new(
            "test-env",
            "container-123",
            PathBuf::from("/home/user/project"),
            "alpine:latest",
        );

        assert_eq!(handle.env_id, "test-env");
        assert_eq!(handle.container_id, "container-123");
        assert_eq!(handle.project_root, Path::new("/home/user/project"));
        assert_eq!(handle.mount_path, "/workdir");
        assert_eq!(handle.status, EnvironmentStatus::Creating);
        assert_eq!(handle.image, "alpine:latest");
        assert!(handle.env_vars.is_empty());

        // Created_at should be recent
        let now = Utc::now();
        let diff = now.signed_duration_since(handle.created_at);
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_environment_handle_serialization() {
        // Requirement 2.6: Handle should be serializable to JSON
        let mut handle = EnvironmentHandle::new(
            "test-env",
            "container-123",
            PathBuf::from("/home/user/project"),
            "alpine:latest",
        );

        handle.add_env_vars(
            vec![("KEY".to_string(), "VALUE".to_string())]
                .into_iter()
                .collect(),
        );

        let json = serde_json::to_string(&handle).unwrap();
        assert!(json.contains("\"env_id\":\"test-env\""));
        assert!(json.contains("\"container_id\":\"container-123\""));
        assert!(json.contains("\"mount_path\":\"/workdir\""));
        assert!(json.contains("\"status\":\"creating\""));
        assert!(json.contains("\"KEY\":\"VALUE\""));

        // Deserialize back
        let restored: EnvironmentHandle = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.env_id, handle.env_id);
        assert_eq!(restored.container_id, handle.container_id);
        assert_eq!(restored.status, handle.status);
    }

    #[test]
    fn test_status_updates() {
        let mut handle = EnvironmentHandle::new(
            "test-env",
            "container-123",
            PathBuf::from("/home/user/project"),
            "alpine:latest",
        );

        assert_eq!(handle.status, EnvironmentStatus::Creating);
        assert!(!handle.is_running());
        assert!(!handle.is_error());

        handle.set_status(EnvironmentStatus::Running);
        assert_eq!(handle.status, EnvironmentStatus::Running);
        assert!(handle.is_running());
        assert!(!handle.is_error());

        handle.set_status(EnvironmentStatus::Error("Failed".to_string()));
        assert!(!handle.is_running());
        assert!(handle.is_error());
    }

    #[test]
    fn test_environment_status_serialization() {
        let statuses = vec![
            EnvironmentStatus::Creating,
            EnvironmentStatus::Running,
            EnvironmentStatus::Stopping,
            EnvironmentStatus::Stopped,
            EnvironmentStatus::Error("test error".to_string()),
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let restored: EnvironmentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, restored);
        }

        // Check lowercase serialization
        let json = serde_json::to_string(&EnvironmentStatus::Running).unwrap();
        assert_eq!(json, "\"running\"");
    }

    #[test]
    fn test_env_vars_management() {
        let mut handle = EnvironmentHandle::new(
            "test-env",
            "container-123",
            PathBuf::from("/home/user/project"),
            "alpine:latest",
        );

        assert!(handle.env_vars.is_empty());

        let vars = vec![
            ("VAR1".to_string(), "value1".to_string()),
            ("VAR2".to_string(), "value2".to_string()),
        ]
        .into_iter()
        .collect();

        handle.add_env_vars(vars);
        assert_eq!(handle.env_vars.len(), 2);
        assert_eq!(handle.env_vars.get("VAR1"), Some(&"value1".to_string()));
        assert_eq!(handle.env_vars.get("VAR2"), Some(&"value2".to_string()));

        // Add more vars
        let more_vars = vec![("VAR3".to_string(), "value3".to_string())]
            .into_iter()
            .collect();

        handle.add_env_vars(more_vars);
        assert_eq!(handle.env_vars.len(), 3);
    }
}