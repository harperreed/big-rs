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

    match &cli.command {
        Some(Commands::GenerateHtml) => {
            println!("generate-html: unimplemented");
        }
        Some(Commands::GenerateSlides) => {
            println!("generate-slides: unimplemented");
        }
        Some(Commands::GeneratePptx) => {
            println!("generate-pptx: unimplemented");
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }
}
