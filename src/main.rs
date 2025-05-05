// ABOUTME: Main entry point for the big-slides program.
// ABOUTME: Provides CLI interface and executes commands from the library.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate HTML from markdown
    GenerateHtml,
    
    /// Generate slides (images) from HTML
    GenerateSlides,
    
    /// Generate PPTX from slides
    GeneratePptx,
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::GenerateHtml) => {
            println!("Executing generate-html command...");
            big::generate_html()
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

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
