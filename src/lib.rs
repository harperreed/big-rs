// ABOUTME: Library module for the big-slides program.
// ABOUTME: Contains core functionality for generating HTML, slides, and PPTX files.

use anyhow::{Context, Result};
use comrak::{markdown_to_html, ComrakOptions};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum BigError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to fetch remote resource: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("Invalid resource path: {0}")]
    InvalidResourcePath(String),
}

#[derive(Debug, Clone)]
pub struct ResourceFile {
    path: String,
    is_remote: bool,
}

impl ResourceFile {
    pub fn new(path: &str) -> Self {
        let is_remote = path.starts_with("http://") || path.starts_with("https://");
        Self {
            path: path.to_string(),
            is_remote,
        }
    }

    pub fn content(&self) -> Result<String, BigError> {
        if self.is_remote {
            let content = reqwest::blocking::get(&self.path)
                .map_err(|e| BigError::FetchError(e))?
                .text()
                .map_err(|e| BigError::FetchError(e))?;
            Ok(content)
        } else {
            let content = fs::read_to_string(&self.path)
                .map_err(|e| BigError::FileReadError(e))?;
            Ok(content)
        }
    }

    pub fn tag(&self, tag_type: &str) -> String {
        match tag_type {
            "css" => {
                if self.is_remote {
                    format!(r#"<link rel="stylesheet" href="{}">"#, self.path)
                } else {
                    let content = self.content().unwrap_or_default();
                    format!(r#"<style>{}</style>"#, content)
                }
            }
            "js" => {
                if self.is_remote {
                    format!(r#"<script src="{}"></script>"#, self.path)
                } else {
                    let content = self.content().unwrap_or_default();
                    format!(r#"<script>{}</script>"#, content)
                }
            }
            _ => String::new(),
        }
    }
}

/// Generate HTML from a markdown file
pub fn generate_html(
    markdown_path: &Path,
    css_files: &[ResourceFile],
    js_files: &[ResourceFile],
) -> Result<String> {
    // Read markdown content
    let markdown_content = fs::read_to_string(markdown_path)
        .with_context(|| format!("Failed to read markdown file: {:?}", markdown_path))?;

    // Convert markdown to HTML
    let options = ComrakOptions::default();
    let html_content = markdown_to_html(&markdown_content, &options);

    // Build the full HTML document
    let mut html_doc = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html_doc.push_str("<meta charset=\"UTF-8\">\n");
    html_doc.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html_doc.push_str("<title>Presentation</title>\n");

    // Add CSS
    for css in css_files {
        html_doc.push_str(&css.tag("css"));
        html_doc.push('\n');
    }

    html_doc.push_str("</head>\n<body>\n");

    // Wrap content in div for slides
    html_doc.push_str("<div class=\"slides\">\n");
    html_doc.push_str(&html_content);
    html_doc.push_str("\n</div>\n");

    // Add JavaScript
    for js in js_files {
        html_doc.push_str(&js.tag("js"));
        html_doc.push('\n');
    }

    html_doc.push_str("</body>\n</html>");

    Ok(html_doc)
}

/// Generate slides (images) from HTML
pub fn generate_slides() -> Result<()> {
    // Placeholder for slides generation logic
    Ok(())
}

/// Generate PPTX from slides
pub fn generate_pptx() -> Result<()> {
    // Placeholder for PPTX generation logic
    Ok(())
}