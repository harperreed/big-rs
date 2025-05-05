// ABOUTME: Watch module for monitoring file changes and regenerating outputs
// ABOUTME: Provides file watching and auto-regeneration of HTML, slides and PPTX

use log::{debug, error, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{Debouncer, new_debouncer};
use tiny_http::{Header, Response, Server, StatusCode};

use crate::config::Config as AppConfig;
use crate::errors::{BigError, Result};
use crate::html;
use crate::pptx;
use crate::render;
use crate::resources::ResourceFile;
use crate::utils;

/// Configuration for watch mode
pub struct WatchConfig {
    /// Path to the markdown file to watch
    pub markdown_path: PathBuf,

    /// Output HTML file path
    pub html_output: PathBuf,

    /// Output directory for slide images
    pub slides_output_dir: Option<PathBuf>,

    /// Output PPTX file path
    pub pptx_output: Option<PathBuf>,

    /// CSS files to include
    pub css_files: Vec<ResourceFile>,

    /// JavaScript files to include
    pub js_files: Vec<ResourceFile>,

    /// Whether to embed resources in HTML
    pub embed_resources: bool,

    /// Debounce time in milliseconds
    pub debounce_ms: u64,

    /// Whether to serve the HTML using a local web server
    pub serve: bool,

    /// Port for local web server
    pub port: u16,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            markdown_path: PathBuf::new(),
            html_output: PathBuf::new(),
            slides_output_dir: None,
            pptx_output: None,
            css_files: Vec::new(),
            js_files: Vec::new(),
            embed_resources: true,
            debounce_ms: 500,
            serve: false,
            port: 8080,
        }
    }
}

// Not using this type alias currently, but keeping for future refactoring
#[allow(dead_code)]
type WatchDebouncer = Debouncer<notify::FsEventWatcher, notify_debouncer_full::FileIdMap>;

/// Start a simple HTTP server to serve HTML and related files
fn start_server(html_path: PathBuf, port: u16) -> Result<()> {
    let server = Server::http(format!("0.0.0.0:{}", port))
        .map_err(|e| BigError::WatchError(format!("Failed to start HTTP server: {}", e)))?;

    // Get the directory containing the HTML file
    let html_dir = html_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let html_file_name = html_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let server_arc = Arc::new(server);
    let server_thread = server_arc.clone();

    thread::spawn(move || {
        info!("HTTP server listening on http://localhost:{}", port);
        println!("HTTP server listening on http://localhost:{}", port);

        for request in server_thread.incoming_requests() {
            let url_path = request.url();

            // Try to map the URL path to a file path
            let file_path = if url_path == "/" {
                html_dir.join(&html_file_name)
            } else {
                let clean_path = url_path.trim_start_matches('/');
                html_dir.join(clean_path)
            };

            debug!("Request for {:?} -> {:?}", url_path, file_path);

            // Check if the file exists and isn't a directory
            if file_path.exists() && file_path.is_file() {
                // Try to read the file
                match fs::read(&file_path) {
                    Ok(content) => {
                        // Determine content type based on file extension
                        let content_type = match file_path.extension() {
                            Some(ext) if ext.to_string_lossy() == "html" => "text/html",
                            Some(ext) if ext.to_string_lossy() == "css" => "text/css",
                            Some(ext) if ext.to_string_lossy() == "js" => "application/javascript",
                            Some(ext) if ext.to_string_lossy() == "png" => "image/png",
                            Some(ext)
                                if ext.to_string_lossy() == "jpg"
                                    || ext.to_string_lossy() == "jpeg" =>
                            {
                                "image/jpeg"
                            }
                            _ => "application/octet-stream",
                        };

                        // Create header for content type
                        let header = Header::from_bytes("Content-Type", content_type)
                            .expect("Failed to create content-type header");

                        // Send the response
                        let response = Response::from_data(content).with_header(header);
                        if let Err(e) = request.respond(response) {
                            error!("Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to read file {:?}: {}", file_path, e);
                        let response = Response::from_string(format!("Failed to read file: {}", e))
                            .with_status_code(StatusCode(500));
                        let _ = request.respond(response);
                    }
                }
            } else {
                // File not found
                let response =
                    Response::from_string("404 Not Found").with_status_code(StatusCode(404));
                let _ = request.respond(response);
            }
        }
    });

    Ok(())
}

/// Starts watching a markdown file and auto-regenerates outputs when changes occur
pub fn watch_markdown(config: WatchConfig, app_config: &AppConfig) -> Result<()> {
    // Validate input file exists
    utils::validate_file_exists(&config.markdown_path)?;

    // Ensure parent directory for output exists
    utils::ensure_parent_directory_exists(&config.html_output)?;

    // If slides output is specified, ensure that directory exists
    if let Some(slides_dir) = &config.slides_output_dir {
        utils::ensure_directory_exists(slides_dir)?;
        utils::validate_directory_writable(slides_dir)?;
    }

    // If PPTX output is specified, ensure parent directory exists
    if let Some(pptx_output) = &config.pptx_output {
        utils::ensure_parent_directory_exists(pptx_output)?;
    }

    // Initial generation
    regenerate_outputs(&config, app_config)?;

    // Start local server if requested
    if config.serve {
        start_server(config.html_output.clone(), config.port)?;
    }

    // Create a channel to receive file system events
    let (tx, rx) = mpsc::channel();

    // Create debouncer for file system events
    let mut debouncer = new_debouncer(Duration::from_millis(config.debounce_ms), None, tx)
        .map_err(|e| BigError::WatchError(format!("Failed to create file watcher: {}", e)))?;

    // Get the directory containing the markdown file
    let watch_path = match config.markdown_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."), // If no parent (just a filename) or empty parent, use current directory
    };

    // Ensure we're using an absolute path for watching
    let abs_watch_path = if watch_path.is_absolute() {
        watch_path.to_path_buf()
    } else {
        utils::get_absolute_path(watch_path)?
    };

    debug!("Watching absolute path: {:?}", abs_watch_path);

    // Add a path to be watched
    debouncer
        .watcher()
        .watch(&abs_watch_path, RecursiveMode::Recursive)
        .map_err(|e| {
            BigError::WatchError(format!(
                "Failed to start watching directory: {} about {:?}",
                e,
                [abs_watch_path]
            ))
        })?;

    info!("Watching for changes in {:?}", watch_path);
    println!(
        "Watching for changes in {:?} (Press Ctrl+C to stop)",
        watch_path
    );

    // Track seen events to avoid duplicate processing
    let mut last_processed = std::time::Instant::now();

    // Process events
    for result in rx {
        match result {
            Ok(events) => {
                // Filter out events for the markdown file or related resources
                let relevant_changes = events.iter().any(|event| {
                    if event.paths.is_empty() {
                        debug!("Received event with no paths: {:?}", event);
                        return false;
                    }

                    // DebouncedEvent has paths (multiple) instead of path
                    let relevant_paths = event.paths.iter().any(|path| {
                        let is_relevant = is_relevant_path(path, &config);
                        if is_relevant {
                            debug!("Detected relevant change in {:?}", path);
                        }
                        is_relevant
                    });

                    relevant_paths
                });

                // Only regenerate if there are relevant changes and enough time has passed
                let now = std::time::Instant::now();
                if relevant_changes
                    && now.duration_since(last_processed)
                        > Duration::from_millis(config.debounce_ms)
                {
                    match regenerate_outputs(&config, app_config) {
                        Ok(_) => {
                            info!("Regenerated outputs successfully");
                            last_processed = now;
                        }
                        Err(e) => error!("Failed to regenerate outputs: {}", e),
                    }
                }
            }
            Err(e) => error!("Watch error: {:?}", e),
        }
    }

    Ok(())
}

/// Checks if a path is relevant to watch (markdown file or resource)
fn is_relevant_path(path: &Path, config: &WatchConfig) -> bool {
    // Try to get absolute paths for comparison
    let path_abs = match utils::get_absolute_path(path) {
        Ok(p) => p,
        Err(_) => return false, // If we can't resolve the path, it's not relevant
    };

    let md_path_abs = match utils::get_absolute_path(&config.markdown_path) {
        Ok(p) => p,
        Err(_) => config.markdown_path.clone(), // Fall back to the original path
    };

    // Always include the main markdown file
    if path_abs == md_path_abs || path == config.markdown_path {
        return true;
    }

    // Check if it's a local CSS or JS file
    let path_str = path.to_string_lossy().to_string();
    let path_abs_str = path_abs.to_string_lossy().to_string();

    for css in &config.css_files {
        if !css.is_remote {
            // Try both the original path and absolute path
            if css.path == path_str || css.path == path_abs_str {
                return true;
            }
        }
    }

    for js in &config.js_files {
        if !js.is_remote {
            // Try both the original path and absolute path
            if js.path == path_str || js.path == path_abs_str {
                return true;
            }
        }
    }

    // Check file extension
    match path.extension() {
        Some(ext) => {
            let ext_str = ext.to_string_lossy().to_lowercase();
            ext_str == "md" || ext_str == "css" || ext_str == "js"
        }
        None => false,
    }
}

/// Regenerate all outputs based on the current state of the markdown file
fn regenerate_outputs(config: &WatchConfig, app_config: &AppConfig) -> Result<()> {
    info!("Regenerating outputs...");

    // Generate HTML
    let html_content = html::generate_html(
        &config.markdown_path,
        &config.css_files,
        &config.js_files,
        config.embed_resources,
    )?;

    // Write HTML to file
    html::write_html_to_file(&html_content, &config.html_output)?;
    info!("HTML regenerated: {:?}", config.html_output);

    // Generate slides if output directory is specified
    if let Some(slides_dir) = &config.slides_output_dir {
        // Create render configuration
        let render_config = app_config.get_render_config(
            None, // Use defaults from app_config
            None, None, None, None,
        );

        // Generate slides (screenshots)
        let output_files =
            render::generate_slides(&config.html_output, slides_dir, &render_config)?;

        info!(
            "Slides regenerated: {} slides in {:?}",
            output_files.len(),
            slides_dir
        );

        // Generate PPTX if output path is specified
        if let Some(pptx_output) = &config.pptx_output {
            // Create PPTX configuration
            let pptx_config = app_config.get_pptx_config(
                None, // Use defaults from app_config
                None, None,
            );

            // Generate PowerPoint presentation from images
            pptx::generate_pptx(slides_dir, pptx_output, &pptx_config)?;

            info!("PPTX regenerated: {:?}", pptx_output);
        }
    }

    Ok(())
}
