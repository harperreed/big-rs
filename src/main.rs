// ABOUTME: Main entry point for the big-slides program.
// ABOUTME: Provides CLI interface and executes commands from the library.

use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    #[arg(long, default_value = "1280")]
    width: u32,

    /// Height of the slides in pixels
    #[arg(long, default_value = "720")]
    height: u32,
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();
    
    let cli = Cli::parse();

    let result: Result<(), Box<dyn std::error::Error>> = match &cli.command {
        Some(Commands::GenerateHtml(args)) => {
            println!("Executing generate-html command...");
            
            // Convert CSS files to ResourceFile structs
            let css_files: Vec<big::ResourceFile> = args.css.as_ref()
                .map(|files| files.iter().map(|path| big::ResourceFile::new(path)).collect())
                .unwrap_or_default();

            // Convert JS files to ResourceFile structs
            let js_files: Vec<big::ResourceFile> = args.js.as_ref()
                .map(|files| files.iter().map(|path| big::ResourceFile::new(path)).collect())
                .unwrap_or_default();
            
            // Generate HTML content
            let html_content = big::generate_html(&args.input, &css_files, &js_files)?;
            
            // Write the HTML content to the output file
            fs::write(&args.output, html_content)
                .map_err(|e| anyhow::anyhow!("Failed to write output file: {}", e))?;
            
            println!("HTML generated successfully: {:?}", args.output);
            Ok(())
        }
        Some(Commands::GenerateSlides(args)) => {
            println!("Executing generate-slides command...");
            
            // Create output directory if it doesn't exist
            if !args.output_dir.exists() {
                fs::create_dir_all(&args.output_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;
            }
            
            // Generate slides (screenshots)
            big::generate_slides(
                &args.input,
                &args.output_dir,
                &args.base_name,
                &args.format,
                args.width,
                args.height,
            )?;
            
            println!("Slides generated successfully: {:?}", args.output_dir);
            Ok(())
        }
        Some(Commands::GeneratePptx(args)) => {
            println!("Executing generate-pptx command...");
            
            // Generate PowerPoint presentation from images
            big::generate_pptx(
                &args.input_dir,
                &args.output,
                &args.pattern,
                &args.title
            )?;
            
            println!("PPTX generated successfully: {:?}", args.output);
            Ok(())
        }
        None => {
            println!("No command specified. Use --help for usage information.");
            Ok(())
        }
    };

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
