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
    let markdown_content = fs::read_to_string(markdown_path).map_err(BigError::FileReadError)?;

    // Parse frontmatter and content
    let (title, _author, _date, content) = parse_frontmatter(&markdown_content);

    // Process content to handle "#" headers as slide breaks
    let processed_content = process_content_for_slides(content);

    // Convert markdown to HTML
    let options = ComrakOptions::default();
    let html_content = markdown_to_html(&processed_content, &options);

    // Build the full HTML document
    let mut html_doc = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html_doc.push_str("<meta charset=\"UTF-8\">\n");
    html_doc
        .push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=0\">\n");
    html_doc.push_str(&format!("<title>{}</title>\n", title));

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

    // Process raw HTML to split into slides
    let slides = extract_slides(&html_content);
    html_doc.push_str("<div class=\"slides\">\n");
    for slide in slides {
        html_doc.push_str("<div>");
        html_doc.push_str(&slide);
        html_doc.push_str("</div>\n");
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

/// Parse frontmatter in the format: % Title\n% Author\n% Date
fn parse_frontmatter(content: &str) -> (String, String, String, String) {
    let lines: Vec<&str> = content.lines().collect();

    // Default values
    let mut title = "Presentation".to_string();
    let mut author = "".to_string();
    let mut date = "".to_string();

    // Check if we have frontmatter
    if lines.len() >= 3 && lines[0].starts_with("% ") {
        title = lines[0].trim_start_matches("% ").trim().to_string();
        if lines.len() >= 4 && lines[1].starts_with("% ") {
            author = lines[1].trim_start_matches("% ").trim().to_string();
            if lines[2].starts_with("% ") {
                date = lines[2].trim_start_matches("% ").trim().to_string();

                // Find the first empty line after frontmatter
                let mut start_idx = 3;
                while start_idx < lines.len() && !lines[start_idx].trim().is_empty() {
                    start_idx += 1;
                }
                start_idx = std::cmp::min(start_idx + 1, lines.len());

                // Return the rest of the content
                return (title, author, date, lines[start_idx..].join("\n"));
            }
        }
    }

    // If we didn't find frontmatter, return the original content
    (title, author, date, content.to_string())
}

/// Process content to convert headers to slide breaks
fn process_content_for_slides(content: String) -> String {
    // Replace "#" that are not part of headings with "!--HASH--!"
    let content = content.replace("\\#", "!--HASH--!");

    // Split content by lines to properly handle headers
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut is_first_section = true;

    for line in lines {
        // If line starts with exactly one "#" (level 1 header), add a slide break
        if line.trim().starts_with("# ") && !is_first_section {
            result.push_str("\n\n---\n\n");
        }

        result.push_str(line);
        result.push('\n');

        if line.trim().starts_with("# ") {
            is_first_section = false;
        }
    }

    // Restore any escaped hashes
    result.replace("!--HASH--!", "#")
}

/// Extract individual slides from the HTML content
fn extract_slides(html_content: &str) -> Vec<String> {
    // Split by horizontal rule
    let parts: Vec<&str> = html_content.split("<hr />").collect();

    // Convert to owned strings
    parts.into_iter().map(|p| p.trim().to_string()).collect()
}

/// Utility function to write HTML content to a file
pub fn write_html_to_file(html_content: &str, output_path: &Path) -> Result<()> {
    info!("Writing HTML to file: {:?}", output_path);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(BigError::FileReadError)?;
        }
    }

    // Write the HTML content to the output file
    fs::write(output_path, html_content).map_err(BigError::FileReadError)?;

    Ok(())
}
