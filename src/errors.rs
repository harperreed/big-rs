// ABOUTME: Error types for the big-slides application
// ABOUTME: Provides structured error handling for each stage of the pipeline

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BigError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to fetch remote resource: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("Invalid resource path: {0}")]
    InvalidResourcePath(String),

    #[error("Markdown conversion error: {0}")]
    MarkdownError(String),

    #[error("HTML generation error: {0}")]
    HtmlError(String),

    #[error("Headless browser error: {message}")]
    BrowserError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Failed to capture screenshot: {0}")]
    ScreenshotError(String),

    #[error("PPTX generation error: {0}")]
    PptxError(String),

    #[error("Input validation error: {0}")]
    ValidationError(String),

    #[error("Browser not found. Make sure Chrome/Chromium is installed.")]
    BrowserNotFound,

    #[error("Path not found: {0}")]
    PathNotFoundError(PathBuf),

    #[error("No slides found matching pattern: {0}")]
    NoSlidesFoundError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Watch error: {0}")]
    WatchError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

// Implement conversion from anyhow::Error to our BigError
impl From<anyhow::Error> for BigError {
    fn from(err: anyhow::Error) -> Self {
        BigError::UnknownError(err.to_string())
    }
}

// Implement conversion from zip errors
impl From<zip::result::ZipError> for BigError {
    fn from(err: zip::result::ZipError) -> Self {
        BigError::PptxError(format!("ZIP operation failed: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, BigError>;
