// ABOUTME: Library module for the big-slides program.
// ABOUTME: Contains core functionality for generating HTML, slides, and PPTX files.

// Reexport modules
pub mod config;
pub mod errors;
pub mod html;
pub mod pptx;
pub mod render;
pub mod resources;
pub mod utils;
pub mod watch;

// Reexport common types and functions
pub use config::Config;
pub use errors::{BigError, Result};
pub use html::{generate_html, write_html_to_file};
pub use pptx::{PptxConfig, find_slide_images, generate_pptx};
pub use render::{RenderConfig, generate_slides};
pub use resources::ResourceFile;
pub use watch::{WatchConfig, watch_markdown};

#[cfg(test)]
mod tests;
