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
    GenerateSlides,
    
    /// Generate PPTX from slides
    GeneratePptx,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let result = match &cli.command {
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
        Some(Commands::GenerateSlides) => {
            println!("Executing generate-slides command...");
            big::generate_slides()
        }
        Some(Commands::GeneratePptx) => {
            println!("Executing generate-pptx command...");
            big::generate_pptx()
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
