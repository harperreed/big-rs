// ABOUTME: Main entry point for the big-slides program.
// ABOUTME: Provides CLI interface and executes commands from the library.

use clap::{Args, Parser, Subcommand};
use log::{error, info};
use std::path::PathBuf;

use big::errors::BigError;
use big::errors::Result as BigResult;
use big::utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to browser executable (overrides BROWSER_PATH environment variable)
    #[arg(long)]
    browser_path: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate HTML from markdown
    GenerateHtml(GenerateHtmlArgs),

    /// Generate slides (images) from HTML
    GenerateSlides(GenerateSlidesArgs),

    /// Generate PPTX from slides
    GeneratePptx(GeneratePptxArgs),

    /// Watch for changes and auto-regenerate outputs
    Watch(WatchArgs),
}

#[derive(Args)]
struct GenerateHtmlArgs {
    /// Path to the markdown file
    #[arg(short, long)]
    input: PathBuf,

    /// Path to output HTML file
    #[arg(short, long)]
    output: PathBuf,

    /// CSS files to include (local paths or URLs)
    #[arg(long, value_delimiter = ',')]
    css: Option<Vec<String>>,

    /// JavaScript files to include (local paths or URLs)
    #[arg(long, value_delimiter = ',')]
    js: Option<Vec<String>>,

    /// Mode for CSS/JS: 'embed' to embed content or 'link' to reference
    #[arg(long, default_value = "embed")]
    mode: String,
}

#[derive(Args)]
struct GenerateSlidesArgs {
    /// Path to the HTML file to render
    #[arg(short, long)]
    input: PathBuf,

    /// Directory to output slide images
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Base filename for slides (will be appended with slide number)
    #[arg(long, default_value = "slide")]
    base_name: String,

    /// Format for the slide images (png, jpeg)
    #[arg(long, default_value = "png")]
    format: String,

    /// Width of the slides in pixels
    #[arg(long, default_value = "1920")]
    width: u32,

    /// Height of the slides in pixels
    #[arg(long, default_value = "1080")]
    height: u32,

    /// Timeout in milliseconds for browser operations
    #[arg(long)]
    timeout_ms: Option<u64>,
}

#[derive(Args)]
struct GeneratePptxArgs {
    /// Directory containing slide images
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Output PPTX file path
    #[arg(short, long)]
    output: PathBuf,

    /// Pattern to match slide images (e.g., "slide_*.png")
    #[arg(long, default_value = "*.png")]
    pattern: String,

    /// Title for the presentation
    #[arg(long, default_value = "Presentation")]
    title: String,

    /// Aspect ratio (16:9 or 4:3)
    #[arg(long, default_value = "16:9")]
    aspect_ratio: String,
}

#[derive(Args)]
struct WatchArgs {
    /// Path to the markdown file to watch
    #[arg(short, long)]
    input: PathBuf,

    /// Path to output HTML file
    #[arg(short, long)]
    output: PathBuf,

    /// CSS files to include (local paths or URLs)
    #[arg(long, value_delimiter = ',')]
    css: Option<Vec<String>>,

    /// JavaScript files to include (local paths or URLs)
    #[arg(long, value_delimiter = ',')]
    js: Option<Vec<String>>,

    /// Mode for CSS/JS: 'embed' to embed content or 'link' to reference
    #[arg(long, default_value = "embed")]
    mode: String,

    /// Directory to output slide images (optional)
    #[arg(long)]
    slides_dir: Option<PathBuf>,

    /// Output PPTX file path (optional)
    #[arg(long)]
    pptx_output: Option<PathBuf>,

    /// Debounce time in milliseconds
    #[arg(long, default_value = "500")]
    debounce_ms: u64,

    /// Start a local web server to serve the HTML
    #[arg(long)]
    serve: bool,

    /// Port for local web server
    #[arg(long, default_value = "8080")]
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logger
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new().filter_level(log_level).init();

    // Load configuration
    let mut config = big::Config::from_env();

    // Override with command line options
    if let Some(browser_path) = cli.browser_path {
        config.browser_path = Some(browser_path);
    }

    // Execute command
    let result = match &cli.command {
        Some(Commands::GenerateHtml(args)) => generate_html(args, &config),
        Some(Commands::GenerateSlides(args)) => generate_slides(args, &config),
        Some(Commands::GeneratePptx(args)) => generate_pptx(args, &config),
        Some(Commands::Watch(args)) => watch(args, &config),
        None => {
            println!("No command specified. Use --help for usage information.");
            Ok(())
        }
    };

    // Handle errors
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            error!("Error: {}", e);

            // Provide additional context for certain errors
            match e {
                BigError::BrowserNotFound => {
                    eprintln!("Error: Browser not found. Make sure Chrome/Chromium is installed.");
                    eprintln!(
                        "You can specify a browser path with --browser-path or the BROWSER_PATH environment variable."
                    );
                }
                BigError::PathNotFoundError(path) => {
                    eprintln!("Error: Path not found: {:?}", path);
                    eprintln!("Please check that the file or directory exists and is accessible.");
                }
                BigError::ValidationError(msg) => {
                    eprintln!("Error: {}", msg);
                    eprintln!("Please check your input arguments and try again.");
                }
                BigError::WatchError(msg) => {
                    eprintln!("Error watching for changes: {}", msg);
                    eprintln!("Please check file permissions and try again.");
                }
                _ => {
                    eprintln!("Error: {}", e);
                }
            }

            std::process::exit(1);
        }
    }
}

/// Execute the generate-html command
fn generate_html(args: &GenerateHtmlArgs, config: &big::Config) -> BigResult<()> {
    info!("Executing generate-html command...");

    // Validate input file exists
    utils::validate_file_exists(&args.input)?;

    // Ensure parent directory for output exists
    utils::ensure_parent_directory_exists(&args.output)?;

    // Parse mode flag
    let embed_resources = args.mode.to_lowercase() != "link";

    // Convert CSS files to ResourceFile structs
    let css_files: Vec<big::ResourceFile> = match args.css.as_ref() {
        Some(files) if !files.is_empty() => {
            // User-specified CSS files
            files
                .iter()
                .map(|path| big::ResourceFile::new(path))
                .collect()
        }
        _ => {
            // No CSS specified, use default
            info!(
                "No CSS files specified, using default CSS: {}",
                config.default_css
            );
            vec![big::ResourceFile::new(&config.default_css)]
        }
    };

    // Convert JS files to ResourceFile structs
    let js_files: Vec<big::ResourceFile> = match args.js.as_ref() {
        Some(files) if !files.is_empty() => {
            // User-specified JS files
            files
                .iter()
                .map(|path| big::ResourceFile::new(path))
                .collect()
        }
        _ => {
            // No JS specified, use default
            info!(
                "No JavaScript files specified, using default JS: {}",
                config.default_js
            );
            vec![big::ResourceFile::new(&config.default_js)]
        }
    };

    // Generate HTML content
    let html_content =
        big::html::generate_html(&args.input, &css_files, &js_files, embed_resources)?;

    // Write the HTML content to the output file
    big::html::write_html_to_file(&html_content, &args.output)?;

    info!("HTML generated successfully: {:?}", args.output);
    println!("HTML generated successfully: {:?}", args.output);
    Ok(())
}

/// Execute the generate-slides command
fn generate_slides(args: &GenerateSlidesArgs, config: &big::Config) -> BigResult<()> {
    info!("Executing generate-slides command...");

    // Validate input file exists
    utils::validate_file_exists(&args.input)?;

    // Ensure output directory exists
    utils::ensure_directory_exists(&args.output_dir)?;

    // Validate output directory is writable
    utils::validate_directory_writable(&args.output_dir)?;

    // Create render configuration
    let render_config = config.get_render_config(
        Some(args.width),
        Some(args.height),
        Some(args.format.clone()),
        Some(args.base_name.clone()),
        args.timeout_ms,
    );

    // Generate slides (screenshots)
    let output_files = big::render::generate_slides(&args.input, &args.output_dir, &render_config)?;

    info!("Generated {} slides", output_files.len());
    println!("Slides generated successfully: {:?}", args.output_dir);
    Ok(())
}

/// Execute the generate-pptx command
fn generate_pptx(args: &GeneratePptxArgs, config: &big::Config) -> BigResult<()> {
    info!("Executing generate-pptx command...");

    // Validate input directory exists
    utils::validate_directory_exists(&args.input_dir)?;

    // Ensure parent directory for output exists
    utils::ensure_parent_directory_exists(&args.output)?;

    // Create PPTX configuration
    let pptx_config = config.get_pptx_config(
        Some(args.title.clone()),
        Some(args.pattern.clone()),
        Some(args.aspect_ratio.clone()),
    );

    // Generate PowerPoint presentation from images
    big::pptx::generate_pptx(&args.input_dir, &args.output, &pptx_config)?;

    info!("PPTX generated successfully: {:?}", args.output);
    println!("PPTX generated successfully: {:?}", args.output);
    Ok(())
}

/// Execute the watch command
fn watch(args: &WatchArgs, config: &big::Config) -> BigResult<()> {
    info!("Executing watch command...");

    // Validate input file exists
    utils::validate_file_exists(&args.input)?;

    // Ensure parent directory for output exists
    utils::ensure_parent_directory_exists(&args.output)?;

    // If slides output is specified, ensure that directory exists
    if let Some(slides_dir) = &args.slides_dir {
        utils::ensure_directory_exists(slides_dir)?;
        utils::validate_directory_writable(slides_dir)?;
    }

    // If PPTX output is specified, ensure parent directory exists
    if let Some(pptx_output) = &args.pptx_output {
        utils::ensure_parent_directory_exists(pptx_output)?;
    }

    // Parse mode flag
    let embed_resources = args.mode.to_lowercase() != "link";

    // Convert CSS files to ResourceFile structs
    let css_files: Vec<big::ResourceFile> = match args.css.as_ref() {
        Some(files) if !files.is_empty() => {
            // User-specified CSS files
            files
                .iter()
                .map(|path| big::ResourceFile::new(path))
                .collect()
        }
        _ => {
            // No CSS specified, use default
            info!(
                "No CSS files specified, using default CSS: {}",
                config.default_css
            );
            vec![big::ResourceFile::new(&config.default_css)]
        }
    };

    // Convert JS files to ResourceFile structs
    let js_files: Vec<big::ResourceFile> = match args.js.as_ref() {
        Some(files) if !files.is_empty() => {
            // User-specified JS files
            files
                .iter()
                .map(|path| big::ResourceFile::new(path))
                .collect()
        }
        _ => {
            // No JS specified, use default
            info!(
                "No JavaScript files specified, using default JS: {}",
                config.default_js
            );
            vec![big::ResourceFile::new(&config.default_js)]
        }
    };

    // Create watch configuration
    let watch_config = big::WatchConfig {
        markdown_path: args.input.clone(),
        html_output: args.output.clone(),
        slides_output_dir: args.slides_dir.clone(),
        pptx_output: args.pptx_output.clone(),
        css_files,
        js_files,
        embed_resources,
        debounce_ms: args.debounce_ms,
        serve: args.serve,
        port: args.port,
    };

    // Start watching
    big::watch_markdown(watch_config, config)
}
