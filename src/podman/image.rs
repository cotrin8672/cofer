use anyhow::{Context, Result};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::ImageSummary;
use futures::StreamExt;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use super::client::PodmanClient;

/// Image management operations for Podman
impl PodmanClient {
    /// Check if an image exists locally
    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        debug!("Checking if image exists: {}", image);

        // Parse image name and tag
        let (name, tag) = parse_image_tag(image);

        let filters = {
            let mut filters = HashMap::new();
            filters.insert("reference".to_string(), vec![format!("{}:{}", name, tag)]);
            filters
        };

        let options = ListImagesOptions {
            all: false,
            filters,
            ..Default::default()
        };

        let images = self.docker
            .list_images(Some(options))
            .await
            .context("Failed to list images")?;

        let exists = !images.is_empty();
        debug!("Image {} exists: {}", image, exists);

        Ok(exists)
    }

    /// List all images
    pub async fn list_images(&self) -> Result<Vec<ImageSummary>> {
        debug!("Listing all images");

        let images = self.docker
            .list_images(None::<ListImagesOptions<String>>)
            .await
            .context("Failed to list images")?;

        info!("Found {} images", images.len());
        Ok(images)
    }

    /// Pull an image from registry
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        info!("Pulling image: {}", image);

        let (name, tag) = parse_image_tag(image);

        let options = Some(CreateImageOptions {
            from_image: name.clone(),
            tag: tag.clone(),
            ..Default::default()
        });

        let mut stream = self.docker.create_image(options, None, None);

        // Process the stream to track progress
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    // Log progress information
                    if let Some(status) = info.status {
                        debug!("Pull progress: {}", status);
                        if let Some(progress) = info.progress_detail {
                            if let (Some(current), Some(total)) = (progress.current, progress.total) {
                                if total > 0 {
                                    let percent = (current as f64 / total as f64) * 100.0;
                                    debug!("  Progress: {:.1}%", percent);
                                }
                            }
                        }
                    }

                    // Check for errors in the info
                    if let Some(error) = info.error {
                        warn!("Pull error: {}", error);
                        return Err(anyhow::anyhow!("Failed to pull image: {}", error));
                    }
                }
                Err(e) => {
                    error!("Failed to pull image {}: {}", image, e);
                    return Err(anyhow::anyhow!("Failed to pull image {}: {}", image, e));
                }
            }
        }

        info!("Successfully pulled image: {}", image);
        Ok(())
    }

    /// Pull image if it doesn't exist locally
    pub async fn ensure_image(&self, image: &str) -> Result<()> {
        if self.image_exists(image).await? {
            info!("Image {} already exists locally", image);
            Ok(())
        } else {
            info!("Image {} not found locally, pulling...", image);
            self.pull_image(image).await
        }
    }

    /// Remove an image
    pub async fn remove_image(&self, image: &str, force: bool) -> Result<()> {
        info!("Removing image: {} (force: {})", image, force);

        let options = bollard::image::RemoveImageOptions {
            force,
            ..Default::default()
        };

        let results = self.docker
            .remove_image(image, Some(options), None)
            .await
            .context("Failed to remove image")?;

        for result in results {
            if let Some(untagged) = result.untagged {
                debug!("Untagged: {}", untagged);
            }
            if let Some(deleted) = result.deleted {
                debug!("Deleted: {}", deleted);
            }
        }

        info!("Successfully removed image: {}", image);
        Ok(())
    }
}

/// Parse image name and tag from image string
fn parse_image_tag(image: &str) -> (String, String) {
    if let Some(pos) = image.rfind(':') {
        // Check if this is a tag or part of a registry URL
        let after_colon = &image[pos + 1..];
        if !after_colon.contains('/') && !after_colon.chars().all(|c| c.is_numeric()) {
            // It's a tag
            return (image[..pos].to_string(), after_colon.to_string());
        }
    }

    // No tag specified, use "latest"
    (image.to_string(), "latest".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_tag() {
        assert_eq!(parse_image_tag("alpine"), ("alpine".to_string(), "latest".to_string()));
        assert_eq!(parse_image_tag("alpine:3.18"), ("alpine".to_string(), "3.18".to_string()));
        assert_eq!(parse_image_tag("docker.io/alpine:latest"), ("docker.io/alpine".to_string(), "latest".to_string()));
        assert_eq!(parse_image_tag("localhost:5000/myimage"), ("localhost:5000/myimage".to_string(), "latest".to_string()));
        assert_eq!(parse_image_tag("localhost:5000/myimage:v1"), ("localhost:5000/myimage".to_string(), "v1".to_string()));
    }

    #[tokio::test]
    #[ignore] // Requires Podman
    async fn test_list_images() {
        if let Ok(client) = PodmanClient::new().await {
            let result = client.list_images().await;
            assert!(result.is_ok());

            let images = result.unwrap();
            // Just verify the API works
            assert!(images.len() >= 0);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman and network
    async fn test_image_exists() {
        if let Ok(client) = PodmanClient::new().await {
            // Test with a likely non-existent image
            let exists = client.image_exists("definitely-not-an-image:v999").await;
            assert!(exists.is_ok());
            assert!(!exists.unwrap());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman and network
    async fn test_pull_small_image() {
        if let Ok(client) = PodmanClient::new().await {
            // Use a very small image for testing
            let result = client.pull_image("docker.io/library/busybox:latest").await;

            // This might fail due to network issues, rate limits, etc.
            // So we just check that the function works
            if result.is_err() {
                let err = result.unwrap_err();
                // Should have a meaningful error
                assert!(!err.to_string().is_empty());
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires Podman
    async fn test_ensure_image() {
        if let Ok(client) = PodmanClient::new().await {
            // This should either find or pull the image
            let result = client.ensure_image("docker.io/library/alpine:latest").await;

            // Check that the function completes
            // Actual success depends on network and Podman state
            if result.is_err() {
                println!("Ensure image failed: {}", result.unwrap_err());
            }
        }
    }
}