// ABOUTME: Configuration module for the big-slides application
// ABOUTME: Provides configuration settings and environment variable handling

// No need for these imports
use crate::pptx::PptxConfig;
use crate::render::RenderConfig;
use std::env;
use std::path::PathBuf;

/// Global configuration for the application
pub struct Config {
    pub browser_path: Option<String>,
    pub html_template_path: Option<PathBuf>,
    pub default_timeout_ms: u64,
    pub embed_resources: bool,
    pub default_css: String,
    pub default_js: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser_path: env::var("BROWSER_PATH").ok(),
            html_template_path: None,
            default_timeout_ms: 30000, // 30 seconds
            embed_resources: true,
            default_css: "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.css"
                .to_string(),
            default_js: "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.js"
                .to_string(),
        }
    }
}

impl Config {
    /// Create a new configuration instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let browser_path = env::var("BROWSER_PATH").ok();
        let html_template_path = env::var("HTML_TEMPLATE_PATH").ok().map(PathBuf::from);
        let default_timeout_ms = env::var("DEFAULT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30000);
        let embed_resources = env::var("EMBED_RESOURCES")
            .ok()
            .map(|s| s.to_lowercase() != "false")
            .unwrap_or(true);

        let default_css = env::var("DEFAULT_CSS").unwrap_or_else(|_| {
            "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.css".to_string()
        });
        let default_js = env::var("DEFAULT_JS").unwrap_or_else(|_| {
            "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.js".to_string()
        });

        Self {
            browser_path,
            html_template_path,
            default_timeout_ms,
            embed_resources,
            default_css,
            default_js,
        }
    }

    /// Get a render configuration with defaults from this config
    pub fn get_render_config(
        &self,
        width: Option<u32>,
        height: Option<u32>,
        format: Option<String>,
        base_name: Option<String>,
        timeout_ms: Option<u64>,
    ) -> RenderConfig {
        RenderConfig {
            width: width.unwrap_or(1280),
            height: height.unwrap_or(720),
            format: format.unwrap_or_else(|| "png".to_string()),
            base_name: base_name.unwrap_or_else(|| "slide".to_string()),
            timeout_ms: timeout_ms.unwrap_or(self.default_timeout_ms),
            browser_path: self.browser_path.clone(),
        }
    }

    /// Get a PPTX configuration with defaults
    pub fn get_pptx_config(
        &self,
        title: Option<String>,
        pattern: Option<String>,
        aspect_ratio: Option<String>,
    ) -> PptxConfig {
        PptxConfig {
            title: title.unwrap_or_else(|| "Presentation".to_string()),
            pattern: pattern.unwrap_or_else(|| "*.png".to_string()),
            aspect_ratio: aspect_ratio.unwrap_or_else(|| "16:9".to_string()),
        }
    }
}
