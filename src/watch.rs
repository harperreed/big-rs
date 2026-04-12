// ABOUTME: Watch module for monitoring file changes and regenerating outputs
// ABOUTME: Provides file watching and auto-regeneration of HTML, slides and PPTX

use log::{debug, error, info};
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{Debouncer, new_debouncer};
use parking_lot::Mutex;
use tiny_http::{Header, Response, Server, StatusCode};
use tungstenite::{Message, accept};

use crate::config::Config as AppConfig;
use crate::diff;
use crate::errors::{BigError, Result};
use crate::html;
use crate::pptx;
use crate::render;
use crate::resources::ResourceFile;
use crate::utils;
use crate::watch_events::WatchEvent;

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

    /// Whether to enable auto-reload functionality
    pub auto_reload: bool,

    /// WebSocket port for auto-reload (defaults to HTTP port + 1)
    pub ws_port: Option<u16>,

    /// Whether to enable TUI dashboard
    pub tui: bool,

    /// Estimated minutes per slide for time calculation
    pub minutes_per_slide: f64,
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
            auto_reload: false,
            ws_port: None,
            tui: false,
            minutes_per_slide: 2.0,
        }
    }
}

// Not using this type alias currently, but keeping for future refactoring
#[allow(dead_code)]
type WatchDebouncer = Debouncer<notify::FsEventWatcher, notify_debouncer_full::FileIdMap>;

/// Structure to manage active WebSocket connections
struct WebSocketManager {
    connections: Vec<WebSocketConnection>,
}

/// Represents a single WebSocket connection
struct WebSocketConnection {
    connection: tungstenite::WebSocket<TcpStream>,
}

impl WebSocketManager {
    fn new() -> Self {
        WebSocketManager {
            connections: Vec::new(),
        }
    }

    fn add_connection(&mut self, stream: TcpStream) -> Result<()> {
        // Keep socket in blocking mode for the handshake
        match accept(
            stream
                .try_clone()
                .map_err(|e| BigError::WatchError(format!("Failed to clone stream: {}", e)))?,
        ) {
            Ok(websocket) => {
                // After handshake, the socket is already in blocking mode
                // We'll leave it that way since tungstenite handles async internally

                self.connections.push(WebSocketConnection {
                    connection: websocket,
                });
                Ok(())
            }
            Err(e) => {
                // Check if this is just a would-block error, which we can ignore
                if format!("{:?}", e).contains("WouldBlock") {
                    debug!("Ignoring temporary WouldBlock error during WebSocket handshake");
                    Ok(())
                } else {
                    Err(BigError::WatchError(format!(
                        "Failed to accept WebSocket connection: {}",
                        e
                    )))
                }
            }
        }
    }

    /// Sends a message to all connected clients
    fn broadcast(&mut self, message: &str) {
        let mut i = 0;
        while i < self.connections.len() {
            match self.connections[i]
                .connection
                .send(Message::Text(message.to_string()))
            {
                Ok(_) => {
                    i += 1;
                }
                Err(_) => {
                    // Remove failed connections
                    self.connections.remove(i);
                }
            }
        }
    }

    /// Cleanup and prepare for shutdown
    fn cleanup(&mut self) {
        for connection in &mut self.connections {
            let _ = connection.connection.close(None);
        }
        self.connections.clear();
    }
}

/// JavaScript code for auto-reload functionality
fn auto_reload_js(ws_port: u16) -> String {
    format!(
        r#"<script>
// Auto-reload WebSocket client
(function() {{
    const ws_port = {};
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${{wsProtocol}}//${{window.location.hostname}}:${{ws_port}}`;
    
    let socket;
    let reconnectInterval;
    
    function connect() {{
        socket = new WebSocket(wsUrl);
        
        socket.onopen = function() {{
            console.log('Connected to auto-reload server');
            clearInterval(reconnectInterval);
        }};
        
        socket.onclose = function() {{
            console.log('Disconnected from auto-reload server. Attempting to reconnect...');
            reconnectInterval = setInterval(connect, 3000);
        }};
        
        socket.onerror = function(error) {{
            console.error('WebSocket error:', error);
            socket.close();
        }};
        
        socket.onmessage = function(event) {{
            console.log('Received message:', event.data);
            try {{
                var msg = JSON.parse(event.data);
                if (msg.action === 'reload') {{
                    if (msg.focus_slide !== undefined && msg.focus_slide !== null) {{
                        sessionStorage.setItem('big_focus_slide', String(msg.focus_slide));
                    }}
                    window.location.reload();
                }}
            }} catch(e) {{
                if (event.data === 'reload') {{
                    window.location.reload();
                }}
            }}
        }};
    }}

    // Navigate to focused slide after reload.
    // big.js initializes on window.onload and exposes window.big.go(),
    // so we poll until it exists rather than using DOMContentLoaded.
    function applyFocusSlide() {{
        var focusSlide = sessionStorage.getItem('big_focus_slide');
        if (focusSlide === null) return;

        function tryNavigate() {{
            if (window.big && typeof window.big.go === 'function') {{
                sessionStorage.removeItem('big_focus_slide');
                var slideIndex = parseInt(focusSlide, 10);
                window.big.go(slideIndex);
            }} else {{
                requestAnimationFrame(tryNavigate);
            }}
        }}
        requestAnimationFrame(tryNavigate);
    }}

    // Run after page load to ensure big.js has had a chance to init
    if (document.readyState === 'complete') {{
        applyFocusSlide();
    }} else {{
        window.addEventListener('load', applyFocusSlide);
    }}

    // Start connection
    connect();
}})();
</script>"#,
        ws_port
    )
}

// Response type for the start_server function
type ServerResponse = (Arc<Server>, Option<Arc<Mutex<WebSocketManager>>>);

/// Start a simple HTTP server to serve HTML and related files
/// Also starts a WebSocket server for auto-reload if requested
fn start_server(
    html_path: PathBuf,
    port: u16,
    auto_reload: bool,
    ws_port: Option<u16>,
    quiet: bool,
) -> Result<ServerResponse> {
    // Start HTTP server
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

    // Start WebSocket server for auto-reload if requested
    let ws_manager = if auto_reload {
        // Calculate WebSocket port if not specified
        let ws_port = ws_port.unwrap_or(port + 1);

        // Create WebSocket manager
        let ws_manager = Arc::new(Mutex::new(WebSocketManager::new()));
        let ws_thread_manager = ws_manager.clone();

        // Start WebSocket server in a separate thread
        thread::spawn(move || {
            match TcpListener::bind(format!("0.0.0.0:{}", ws_port)) {
                Ok(listener) => {
                    info!(
                        "WebSocket server for auto-reload listening on port {}",
                        ws_port
                    );
                    if !quiet {
                        println!(
                            "WebSocket server for auto-reload listening on port {}",
                            ws_port
                        );
                    }

                    // Set the listener to non-blocking mode to avoid freezing on accept
                    if let Err(e) = listener.set_nonblocking(true) {
                        error!(
                            "Failed to set WebSocket listener to non-blocking mode: {}",
                            e
                        );
                        return;
                    }

                    // Accept connections with proper handling of would-block errors
                    loop {
                        match listener.accept() {
                            Ok((stream, addr)) => {
                                debug!("New WebSocket connection from {}", addr);
                                if let Err(e) = ws_thread_manager.lock().add_connection(stream) {
                                    error!("Failed to establish WebSocket connection: {}", e);
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                // This is normal in non-blocking mode - just wait a bit
                                thread::sleep(Duration::from_millis(100));
                            }
                            Err(e) => {
                                error!("Failed to accept WebSocket connection: {}", e);
                                // Sleep to avoid tight loop on persistent errors
                                thread::sleep(Duration::from_millis(500));
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to start WebSocket server: {}", e);
                }
            }
        });

        Some(ws_manager)
    } else {
        None
    };

    // Start HTTP server in a separate thread
    thread::spawn(move || {
        info!("HTTP server listening on http://localhost:{}", port);
        if !quiet {
            println!("HTTP server listening on http://localhost:{}", port);
        }

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

    Ok((server_arc, ws_manager))
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

    // Calculate WebSocket port if auto-reload is enabled
    let ws_port = if config.auto_reload {
        Some(config.ws_port.unwrap_or(config.port + 1))
    } else {
        None
    };

    // Initial generation
    regenerate_outputs(&config, app_config, ws_port)?;

    // Start local server if requested
    let ws_manager = if config.serve {
        let (_server, manager) = start_server(
            config.html_output.clone(),
            config.port,
            config.auto_reload,
            ws_port,
            false,
        )?;
        manager
    } else {
        None
    };

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

    // Store previous markdown content for diffing to detect which slide changed
    let mut previous_content = std::fs::read_to_string(&config.markdown_path).unwrap_or_default();

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
                    event.paths.iter().any(|path| {
                        let is_relevant = is_relevant_path(path, &config);
                        if is_relevant {
                            debug!("Detected relevant change in {:?}", path);
                        }
                        is_relevant
                    })
                });

                // Only regenerate if there are relevant changes and enough time has passed
                let now = std::time::Instant::now();
                if relevant_changes
                    && now.duration_since(last_processed)
                        > Duration::from_millis(config.debounce_ms)
                {
                    match regenerate_outputs(&config, app_config, ws_port) {
                        Ok(_) => {
                            info!("Regenerated outputs successfully");
                            last_processed = now;

                            // Send reload message to clients if auto-reload is enabled
                            if let Some(ref ws_manager) = ws_manager {
                                let new_content = std::fs::read_to_string(&config.markdown_path)
                                    .unwrap_or_default();
                                let changed_slide = crate::diff::find_changed_slide(
                                    &previous_content,
                                    &new_content,
                                );
                                let reload_msg = match changed_slide {
                                    Some(slide) => {
                                        crate::watch_events::ReloadMessage::reload_with_focus(slide)
                                    }
                                    None => crate::watch_events::ReloadMessage::reload(),
                                };
                                ws_manager.lock().broadcast(&reload_msg.to_json());
                                previous_content = new_content;
                                info!("Sent reload signal to connected browsers");
                            }
                        }
                        Err(e) => error!("Failed to regenerate outputs: {}", e),
                    }
                }
            }
            Err(e) => error!("Watch error: {:?}", e),
        }
    }

    // Cleanup WebSocket connections before exiting
    if let Some(ws_manager) = ws_manager {
        ws_manager.lock().cleanup();
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
fn regenerate_outputs(
    config: &WatchConfig,
    app_config: &AppConfig,
    ws_port: Option<u16>,
) -> Result<()> {
    info!("Regenerating outputs...");

    // Generate auto-reload script if needed
    let auto_reload_js_script = if config.auto_reload {
        ws_port.map(auto_reload_js)
    } else {
        None
    };

    // Generate HTML
    let html_content = html::generate_html(
        &config.markdown_path,
        &config.css_files,
        &config.js_files,
        config.embed_resources,
        auto_reload_js_script,
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

/// Run the watch backend in a blocking thread, sending events through a tokio channel.
/// This performs initial generation, starts servers, watches for file changes,
/// and sends WatchEvents to the TUI dashboard through the provided sender.
pub(crate) fn run_watch_backend(
    config: WatchConfig,
    app_config: &AppConfig,
    event_tx: boba::tokio::sync::mpsc::UnboundedSender<WatchEvent>,
) {
    // Validate input file exists
    if let Err(e) = utils::validate_file_exists(&config.markdown_path) {
        let _ = event_tx.send(WatchEvent::RegenerationFailed {
            error: format!("Input file validation failed: {}", e),
        });
        return;
    }

    // Calculate WebSocket port if auto-reload is enabled
    let ws_port = if config.auto_reload {
        Some(config.ws_port.unwrap_or(config.port + 1))
    } else {
        None
    };

    // Initial generation
    let _ = event_tx.send(WatchEvent::RegenerationStarted {
        timestamp: std::time::Instant::now(),
    });

    let start_time = std::time::Instant::now();
    match regenerate_outputs(&config, app_config, ws_port) {
        Ok(_) => {
            let markdown_content =
                std::fs::read_to_string(&config.markdown_path).unwrap_or_default();
            let slide_count = diff::count_slides(&markdown_content);
            let slide_titles = diff::extract_slide_titles(&markdown_content);

            let _ = event_tx.send(WatchEvent::RegenerationCompleted {
                slide_count,
                slide_titles: slide_titles.clone(),
                duration_ms: start_time.elapsed().as_millis() as u64,
                changed_slide: None,
            });

            // Send initial stats
            let _ = event_tx.send(WatchEvent::InitialStats {
                slide_count,
                slide_titles,
                markdown_path: config.markdown_path.clone(),
                html_output: config.html_output.clone(),
            });
        }
        Err(e) => {
            let _ = event_tx.send(WatchEvent::RegenerationFailed {
                error: format!("{}", e),
            });
        }
    }

    // Start local server if requested
    let ws_manager = if config.serve {
        match start_server(
            config.html_output.clone(),
            config.port,
            config.auto_reload,
            ws_port,
            true,
        ) {
            Ok((_server, manager)) => {
                let _ = event_tx.send(WatchEvent::ServerStatus {
                    http_port: config.port,
                    ws_port,
                    connected_clients: 0,
                });
                manager
            }
            Err(e) => {
                let _ = event_tx.send(WatchEvent::RegenerationFailed {
                    error: format!("Failed to start server: {}", e),
                });
                None
            }
        }
    } else {
        None
    };

    // Create a channel to receive file system events
    let (tx, rx) = mpsc::channel();

    // Create debouncer for file system events
    let mut debouncer = match new_debouncer(Duration::from_millis(config.debounce_ms), None, tx) {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx.send(WatchEvent::RegenerationFailed {
                error: format!("Failed to create file watcher: {}", e),
            });
            return;
        }
    };

    // Get the directory containing the markdown file
    let watch_path = match config.markdown_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };

    // Ensure we're using an absolute path for watching
    let abs_watch_path = if watch_path.is_absolute() {
        watch_path.to_path_buf()
    } else {
        match utils::get_absolute_path(watch_path) {
            Ok(p) => p,
            Err(e) => {
                let _ = event_tx.send(WatchEvent::RegenerationFailed {
                    error: format!("Failed to resolve watch path: {}", e),
                });
                return;
            }
        }
    };

    // Add a path to be watched
    if let Err(e) = debouncer
        .watcher()
        .watch(&abs_watch_path, RecursiveMode::Recursive)
    {
        let _ = event_tx.send(WatchEvent::RegenerationFailed {
            error: format!("Failed to start watching directory: {}", e),
        });
        return;
    }

    // Track seen events to avoid duplicate processing
    let mut last_processed = std::time::Instant::now();

    // Store previous markdown content for diffing to detect which slide changed
    let mut previous_content = std::fs::read_to_string(&config.markdown_path).unwrap_or_default();

    // Process events from the file watcher
    for result in rx {
        match result {
            Ok(events) => {
                let relevant_changes = events.iter().any(|event| {
                    if event.paths.is_empty() {
                        return false;
                    }
                    event
                        .paths
                        .iter()
                        .any(|path| is_relevant_path(path, &config))
                });

                let now = std::time::Instant::now();
                if relevant_changes
                    && now.duration_since(last_processed)
                        > Duration::from_millis(config.debounce_ms)
                {
                    // Notify about file change
                    let _ = event_tx.send(WatchEvent::FileChanged {
                        path: config.markdown_path.clone(),
                        timestamp: now,
                    });

                    let _ = event_tx.send(WatchEvent::RegenerationStarted { timestamp: now });

                    let regen_start = std::time::Instant::now();
                    match regenerate_outputs(&config, app_config, ws_port) {
                        Ok(_) => {
                            last_processed = now;

                            let new_content =
                                std::fs::read_to_string(&config.markdown_path).unwrap_or_default();
                            let changed_slide =
                                diff::find_changed_slide(&previous_content, &new_content);
                            let slide_count = diff::count_slides(&new_content);
                            let slide_titles = diff::extract_slide_titles(&new_content);

                            let _ = event_tx.send(WatchEvent::RegenerationCompleted {
                                slide_count,
                                slide_titles,
                                duration_ms: regen_start.elapsed().as_millis() as u64,
                                changed_slide,
                            });

                            // Send reload message to clients if auto-reload is enabled
                            if let Some(ref ws_manager) = ws_manager {
                                let reload_msg = match changed_slide {
                                    Some(slide) => {
                                        crate::watch_events::ReloadMessage::reload_with_focus(slide)
                                    }
                                    None => crate::watch_events::ReloadMessage::reload(),
                                };
                                ws_manager.lock().broadcast(&reload_msg.to_json());
                            }

                            previous_content = new_content;
                        }
                        Err(e) => {
                            let _ = event_tx.send(WatchEvent::RegenerationFailed {
                                error: format!("{}", e),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                error!("Watch error: {:?}", e);
            }
        }
    }

    // Cleanup WebSocket connections before exiting
    if let Some(ws_manager) = ws_manager {
        ws_manager.lock().cleanup();
    }
}
