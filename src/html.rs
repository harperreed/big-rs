// ABOUTME: HTML generation module for the big-slides application
// ABOUTME: Converts markdown to HTML with styling and embedded resources

use crate::errors::{BigError, Result};
use crate::resources::ResourceFile;
use comrak::{ComrakOptions, markdown_to_html};
use log::info;
use std::fs;
use std::path::Path;

/// Generate HTML from a markdown file with optional CSS and JS resources
pub fn generate_html(
    markdown_path: &Path,
    css_files: &[ResourceFile],
    js_files: &[ResourceFile],
    embed_resources: bool,
) -> Result<String> {
    info!("Generating HTML from markdown: {:?}", markdown_path);

    // Validate input file exists
    if !markdown_path.exists() {
        return Err(BigError::PathNotFoundError(markdown_path.to_path_buf()));
    }

    // Read markdown content
    let markdown_content =
        fs::read_to_string(markdown_path).map_err(|e| BigError::FileReadError(e))?;

    // Convert markdown to HTML
    let options = ComrakOptions::default();
    let html_content = markdown_to_html(&markdown_content, &options);

    // Build the full HTML document
    let mut html_doc = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html_doc.push_str("<meta charset=\"UTF-8\">\n");
    html_doc
        .push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html_doc.push_str("<title>Presentation</title>\n");

    // Add CSS
    for css in css_files {
        match css.tag("css", embed_resources) {
            Ok(tag) => {
                html_doc.push_str(&tag);
                html_doc.push('\n');
            }
            Err(e) => {
                info!(
                    "Warning: Failed to include CSS resource {}: {}",
                    css.path, e
                );
                // Continue with other resources rather than failing completely
            }
        }
    }

    html_doc.push_str("</head>\n<body>\n");

    // Wrap content in div for slides
    html_doc.push_str("<div class=\"slides\">\n");

    // Process raw HTML to split into slides
    let slides: Vec<&str> = html_content.split("<hr />").collect();
    for slide in slides {
        html_doc.push_str("<div>\n");
        html_doc.push_str(slide);
        html_doc.push_str("\n</div>\n");
    }

    html_doc.push_str("</div>\n");

    // Add JavaScript
    for js in js_files {
        match js.tag("js", embed_resources) {
            Ok(tag) => {
                html_doc.push_str(&tag);
                html_doc.push('\n');
            }
            Err(e) => {
                info!(
                    "Warning: Failed to include JavaScript resource {}: {}",
                    js.path, e
                );
            }
        }
    }

    html_doc.push_str("</body>\n</html>");

    Ok(html_doc)
}

/// Utility function to write HTML content to a file
pub fn write_html_to_file(html_content: &str, output_path: &Path) -> Result<()> {
    info!("Writing HTML to file: {:?}", output_path);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| BigError::FileReadError(e))?;
        }
    }

    // Write the HTML content to the output file
    fs::write(output_path, html_content).map_err(|e| BigError::FileReadError(e))?;

    Ok(())
}
