// ABOUTME: Browser rendering module for the big-slides application
// ABOUTME: Captures screenshots of HTML slides using a headless browser

use crate::errors::{BigError, Result};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{info, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration for browser rendering
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub base_name: String,
    pub timeout_ms: u64,
    pub browser_path: Option<String>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            format: "png".to_string(),
            base_name: "slide".to_string(),
            timeout_ms: 30000, // 30 seconds
            browser_path: None,
        }
    }
}

/// Generate slide images from an HTML file
pub fn generate_slides(
    html_path: &Path,
    output_dir: &Path,
    config: &RenderConfig,
) -> Result<Vec<PathBuf>> {
    info!("Generating slides from HTML: {:?}", html_path);

    // Validate input file exists
    if !html_path.exists() {
        return Err(BigError::PathNotFoundError(html_path.to_path_buf()));
    }

    // Ensure output directory exists
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| BigError::FileReadError(e))?;
    }

    // Configure browser launch options
    let mut launch_options_builder = LaunchOptionsBuilder::default();

    // Set window size and headless mode
    launch_options_builder.window_size(Some((config.width, config.height)));
    launch_options_builder.headless(true);

    // Use custom browser path if specified
    if let Some(browser_path) = &config.browser_path {
        launch_options_builder.path(Some(browser_path.into()));
    } else if let Ok(path) = env::var("BROWSER_PATH") {
        if !path.is_empty() {
            launch_options_builder.path(Some(path.into()));
        }
    }

    let launch_options = launch_options_builder
        .build()
        .map_err(|e| BigError::BrowserError {
            message: format!("Failed to build browser options: {:?}", e),
            source: None,
        })?;

    // Launch headless browser
    info!("Launching headless browser");
    let browser = match Browser::new(launch_options) {
        Ok(browser) => browser,
        Err(e) => {
            let message = format!("Failed to launch browser: {}", e);
            warn!("{}", message);
            return Err(BigError::BrowserError {
                message,
                source: None,
            });
        }
    };

    // Get absolute path for HTML file
    let html_path_abs = fs::canonicalize(html_path).map_err(|e| BigError::FileReadError(e))?;
    let url = format!("file://{}", html_path_abs.to_string_lossy());

    info!("Opening page at URL: {}", url);

    // Create a new tab and navigate to the HTML file
    let tab = browser.new_tab().map_err(|e| BigError::BrowserError {
        message: format!("Failed to create new tab: {}", e),
        source: None,
    })?;

    tab.navigate_to(&url).map_err(|e| BigError::BrowserError {
        message: format!("Failed to navigate to HTML: {}", e),
        source: None,
    })?;

    // Wait for page to load with timeout
    tab.wait_until_navigated()
        .map_err(|e| BigError::BrowserError {
            message: format!("Navigation failed: {}", e),
            source: None,
        })?;

    // Take screenshot of the first slide
    info!("Taking screenshot of slide 1");
    let get_screenshot_format = || match config.format.to_lowercase().as_str() {
        "png" => headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        "jpeg" | "jpg" => headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Jpeg,
        _ => {
            warn!("Unsupported format: {}. Using PNG instead.", config.format);
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png
        }
    };

    let screenshot_data = tab
        .capture_screenshot(get_screenshot_format(), None, None, true)
        .map_err(|e| BigError::ScreenshotError(format!("Failed to capture screenshot: {}", e)))?;

    // Save screenshot to file
    let output_file = output_dir.join(format!("{}_{:04}.{}", config.base_name, 1, config.format));
    fs::write(&output_file, &screenshot_data).map_err(|e| BigError::FileReadError(e))?;

    info!("Screenshot saved to {:?}", output_file);
    let mut output_files = vec![output_file];

    // Try to detect more slides
    if let Ok(total_slides) =
        tab.evaluate("document.querySelectorAll('.slides > div').length", false)
    {
        // Extract the number of slides
        let count_str = format!("{:?}", total_slides.value);
        // Parse the count from the string representation
        let count = count_str
            .trim_matches(|c| c == '"' || c == ' ')
            .parse::<i64>()
            .unwrap_or(1);

        info!("Detected {} slides", count);

        if count > 1 {
            // Iterate through rest of slides
            for i in 1..count {
                // Press right arrow key to advance to next slide
                tab.press_key("ArrowRight")
                    .map_err(|e| BigError::BrowserError {
                        message: format!("Failed to press right arrow key: {}", e),
                        source: None,
                    })?;

                // Wait a bit for transition
                std::thread::sleep(Duration::from_millis(500));

                // Take screenshot
                info!("Taking screenshot of slide {}", i + 1);
                match tab.capture_screenshot(get_screenshot_format(), None, None, true) {
                    Ok(screenshot_data) => {
                        // Save screenshot
                        let output_file = output_dir.join(format!(
                            "{}_{:04}.{}",
                            config.base_name,
                            i + 1,
                            config.format
                        ));
                        fs::write(&output_file, &screenshot_data)
                            .map_err(|e| BigError::FileReadError(e))?;

                        info!("Screenshot saved to {:?}", output_file);
                        output_files.push(output_file);
                    }
                    Err(e) => {
                        // Log the error but continue with other slides
                        warn!("Failed to capture screenshot for slide {}: {}", i + 1, e);
                    }
                }
            }
        }
    }

    Ok(output_files)
}
