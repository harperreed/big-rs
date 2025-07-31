// ABOUTME: Resource handling for the big-slides application
// ABOUTME: Handles local and remote resources like CSS and JavaScript files

use crate::errors::{BigError, Result};
use log::info;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Cache entry for remote resources
#[derive(Debug, Clone)]
struct CacheEntry {
    content: String,
    expires_at: Instant,
}

// Global cache for remote resources with TTL
lazy_static::lazy_static! {
    static ref REMOTE_CACHE: Arc<Mutex<HashMap<String, CacheEntry>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Default TTL for cached remote resources (5 minutes)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);

/// Represents a resource file that can be either local or remote.
#[derive(Debug, Clone)]
pub struct ResourceFile {
    pub path: String,
    pub is_remote: bool,
}

impl ResourceFile {
    /// Create a new ResourceFile from a path string.
    /// The path can be either a local file path or a URL.
    pub fn new(path: &str) -> Self {
        let is_remote = path.starts_with("http://") || path.starts_with("https://");

        let normalized_path = if !is_remote {
            // Try to get absolute path for local files
            match std::path::Path::new(path).canonicalize() {
                Ok(abs_path) => abs_path.to_string_lossy().to_string(),
                Err(_) => path.to_string(), // Fallback to original path if canonicalization fails
            }
        } else {
            path.to_string()
        };

        Self {
            path: normalized_path,
            is_remote,
        }
    }

    /// Get the content of the resource file.
    /// If the file is remote, it will be fetched from the URL.
    /// If the file is local, it will be read from the filesystem.
    pub fn content(&self) -> Result<String> {
        if self.is_remote {
            self.fetch_remote_content()
        } else {
            self.read_local_content()
        }
    }

    /// Fetch content from a remote URL with retry capability and caching
    fn fetch_remote_content(&self) -> Result<String> {
        // First check the cache
        {
            let cache = REMOTE_CACHE.lock().unwrap();
            if let Some(entry) = cache.get(&self.path) {
                if Instant::now() < entry.expires_at {
                    info!("Using cached remote resource: {}", self.path);
                    return Ok(entry.content.clone());
                }
            }
        }

        info!("Fetching remote resource: {}", self.path);

        // Create a client with timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(BigError::FetchError)?;

        // Try up to 3 times with increasing backoff
        let mut retry_delay = 1000; // Start with 1 second
        let mut last_error = None;

        for attempt in 1..=3 {
            match client.get(&self.path).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        let content = response.text().map_err(BigError::FetchError)?;
                        
                        // Cache the successful result
                        {
                            let mut cache = REMOTE_CACHE.lock().unwrap();
                            cache.insert(self.path.clone(), CacheEntry {
                                content: content.clone(),
                                expires_at: Instant::now() + DEFAULT_CACHE_TTL,
                            });
                        }
                        
                        return Ok(content);
                    } else {
                        let status = response.status();
                        last_error =
                            Some(BigError::ValidationError(format!("HTTP error: {}", status)));
                    }
                }
                Err(e) => {
                    last_error = Some(BigError::FetchError(e));
                }
            }

            info!(
                "Fetch attempt {} failed, retrying in {} ms",
                attempt, retry_delay
            );
            std::thread::sleep(Duration::from_millis(retry_delay));
            retry_delay *= 2; // Exponential backoff
        }

        Err(last_error.unwrap_or_else(|| {
            BigError::ValidationError("Unknown error fetching resource".to_string())
        }))
    }

    /// Read content from a local file
    fn read_local_content(&self) -> Result<String> {
        info!("Reading local resource: {}", self.path);
        if !Path::new(&self.path).exists() {
            return Err(BigError::PathNotFoundError(
                Path::new(&self.path).to_path_buf(),
            ));
        }

        fs::read_to_string(&self.path).map_err(BigError::FileReadError)
    }

    /// Generate HTML tag for the resource, either embedding or linking the content.
    /// - tag_type: The type of tag to generate ("css" or "js")
    /// - embed: Whether to embed the content in the tag or link to it
    pub fn tag(&self, tag_type: &str, embed: bool) -> Result<String> {
        if !embed {
            // When explicitly requesting linking, create a link
            Ok(match tag_type {
                "css" => format!(r#"<link rel="stylesheet" href="{}">"#, self.path),
                "js" => format!(r#"<script src="{}"></script>"#, self.path),
                _ => {
                    return Err(BigError::InvalidResourcePath(format!(
                        "Unknown resource type: {}",
                        tag_type
                    )));
                }
            })
        } else {
            // For embedding (both local and remote resources), include the content directly
            let content = self.content()?;
            Ok(match tag_type {
                "css" => format!(r#"<style>{}</style>"#, content),
                "js" => format!(r#"<script>{}</script>"#, content),
                _ => {
                    return Err(BigError::InvalidResourcePath(format!(
                        "Unknown resource type: {}",
                        tag_type
                    )));
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_remote_resource_caching() {
        // Skip this test if we don't have internet connectivity
        let resource = ResourceFile::new("https://httpbin.org/json");
        
        // First fetch
        let start1 = Instant::now();
        let content1 = resource.content();
        let duration1 = start1.elapsed();
        
        assert!(content1.is_ok(), "First fetch should succeed");
        
        // Second fetch - should be cached and faster
        let start2 = Instant::now();
        let content2 = resource.content();
        let duration2 = start2.elapsed();
        
        assert!(content2.is_ok(), "Second fetch should succeed");
        assert_eq!(content1.unwrap(), content2.unwrap(), "Content should be identical");
        
        // Cache should make second fetch significantly faster
        // (allowing some margin for system variability)
        println!("First fetch: {:?}, Second fetch: {:?}", duration1, duration2);
        assert!(duration2 < duration1, "Second fetch should be faster due to caching");
    }
}
