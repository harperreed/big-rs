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
pub use pptx::{find_slide_images, generate_pptx, PptxConfig};
pub use render::{generate_slides, RenderConfig};
pub use resources::ResourceFile;
pub use watch::{watch_markdown, WatchConfig};

#[cfg(test)]
mod tests;
