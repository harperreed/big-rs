// ABOUTME: Library module for the big-slides program.
// ABOUTME: Contains core functionality for generating HTML, slides, and PPTX files.

use anyhow::{Context, Result};
use comrak::{markdown_to_html, ComrakOptions};
use std::fs;
use std::path::{Path, PathBuf};
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
pub fn generate_slides(
    html_path: &Path,
    output_dir: &Path,
    base_name: &str,
    format: &str,
    width: u32,
    height: u32,
) -> Result<Vec<PathBuf>> {
    use headless_chrome::{Browser, LaunchOptions};
    use log::info;
    use std::time::Duration;
    
    info!("Launching headless browser");
    
    // Launch headless Chrome browser
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .window_size(Some((width, height)))
            .headless(true)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Chrome: {}", e))?,
    )
    .map_err(|e| anyhow::anyhow!("Failed to launch Chrome: {}", e))?;
    
    // Get the HTML file URL
    let html_path_abs = fs::canonicalize(html_path)
        .with_context(|| format!("Failed to get absolute path for {:?}", html_path))?;
    let url = format!("file://{}", html_path_abs.to_string_lossy());
    
    info!("Opening page at URL: {}", url);
    
    // Create a new tab and navigate to the HTML file
    let tab = browser.new_tab()
        .map_err(|e| anyhow::anyhow!("Failed to create new tab: {}", e))?;
    
    tab.navigate_to(&url)
        .map_err(|e| anyhow::anyhow!("Failed to navigate to HTML: {}", e))?;
    
    // Wait for page to load
    tab.wait_until_navigated()
        .map_err(|e| anyhow::anyhow!("Navigation failed: {}", e))?;
    
    // Take screenshot of the first slide
    info!("Taking screenshot of slide 1");
    let screenshot_data = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    )
    .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;
    
    // Save screenshot to file
    let output_file = output_dir.join(format!("{}_0001.{}", base_name, format));
    fs::write(&output_file, &screenshot_data)
        .with_context(|| format!("Failed to write screenshot to {:?}", output_file))?;
    
    info!("Screenshot saved to {:?}", output_file);
    let mut output_files = vec![output_file];
    
    // Try to detect more slides
    if let Ok(total_slides) = tab.evaluate("document.querySelectorAll('.slides > *').length", false) {
        // The RemoteObject contains the slides count as a number
        let count_str = format!("{:?}", total_slides.value);
        // Parse the count from debug string representation
        let count = count_str.trim_matches(|c| c == '"' || c == ' ')
            .parse::<i64>()
            .unwrap_or(1);
        info!("Detected {} slides", count);
        
        if count > 1 {
            // Iterate through rest of slides
            for i in 1..count {
                // Press right arrow key to advance to next slide
                tab.press_key("ArrowRight")
                    .map_err(|e| anyhow::anyhow!("Failed to press right arrow key: {}", e))?;
                
                // Wait a bit for transition
                std::thread::sleep(Duration::from_millis(500));
                
                // Take screenshot
                info!("Taking screenshot of slide {}", i + 1);
                let screenshot_data = tab.capture_screenshot(
                    headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                    None,
                    None,
                    true
                )
                .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;
                
                // Save screenshot
                let output_file = output_dir.join(format!("{}_{:04}.{}", base_name, i + 1, format));
                fs::write(&output_file, &screenshot_data)
                    .with_context(|| format!("Failed to write screenshot to {:?}", output_file))?;
                
                info!("Screenshot saved to {:?}", output_file);
                output_files.push(output_file);
            }
        }
    }
    
    Ok(output_files)
}

/// Generate PPTX from slides
pub fn generate_pptx() -> Result<()> {
    // Placeholder for PPTX generation logic
    Ok(())
}