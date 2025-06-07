// ABOUTME: HTML generation module for the big-slides application
// ABOUTME: Converts markdown to HTML with styling and embedded resources

use crate::errors::{BigError, Result};
use crate::resources::ResourceFile;
use comrak::{ComrakOptions, markdown_to_html};
use log::info;
use std::fs;
use std::path::Path;

/// Generate HTML from a markdown file with optional CSS and JS resources
/// This version is for backward compatibility with existing test code
#[allow(dead_code)]
pub fn generate_html_without_reload(
    markdown_path: &Path,
    css_files: &[ResourceFile],
    js_files: &[ResourceFile],
    embed_resources: bool,
) -> Result<String> {
    generate_html(markdown_path, css_files, js_files, embed_resources, None)
}

/// Generate HTML from a markdown file with optional CSS and JS resources
pub fn generate_html(
    markdown_path: &Path,
    css_files: &[ResourceFile],
    js_files: &[ResourceFile],
    embed_resources: bool,
    auto_reload_script: Option<String>,
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

    // Convert markdown to HTML, with options to allow raw HTML
    let mut options = ComrakOptions::default();
    options.render.unsafe_ = true; // Allow raw HTML
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
    // Output directly as divs under body, matching Python renderer
    let slides = extract_slides(&html_content);
    for slide in slides {
        html_doc.push_str("<div>");

        // Extract slide title and remove any paragraph tags for clean output
        let processed_content = extract_slide_content(&slide);

        // Add the processed content
        html_doc.push_str(&processed_content);

        html_doc.push_str("</div>\n");
    }

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

    // Add auto-reload script if provided
    if let Some(script) = auto_reload_script {
        html_doc.push_str(&script);
        html_doc.push('\n');
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

                // Skip optional blank lines after the frontmatter
                let mut start_idx = 3;
                while start_idx < lines.len() && lines[start_idx].trim().is_empty() {
                    start_idx += 1;
                }

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
    let content = content.replace(r"\#", "!--HASH--!");

    // Split content by lines to properly handle headers
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut is_first_section = true;

    for line in lines {
        let trimmed = line.trim();

        // Check for "#Text" (no space) or "# Text" (with space) format
        let is_header = trimmed.starts_with('#')
            && (trimmed.len() == 1 || // Just "#"
                        (trimmed.len() > 1 &&
                         trimmed.chars().nth(1).unwrap() != '#' && // Not "##" (h2)
                         (trimmed.chars().nth(1).unwrap() == ' ' || // "# Text"
                          !trimmed.chars().nth(1).unwrap().is_whitespace()))); // "#Text"

        // Add slide break for headers (except the first one)
        if is_header && !is_first_section {
            result.push_str("\n\n---\n\n");
        }

        // For "#Text" format (no space), convert to "# Text" format for proper markdown rendering
        if is_header && trimmed.len() > 1 && trimmed.chars().nth(1).unwrap() != ' ' {
            // Extract the text part (skipping the #)
            let text_part = &trimmed[1..];
            // Add it as a proper header with space
            result.push_str("# ");
            result.push_str(text_part);
        } else {
            // Add the line as-is to the result
            result.push_str(line);
        }
        result.push('\n');

        // Mark that we've seen a header
        if is_header {
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

/// Extract and clean slide content
fn extract_slide_content(slide: &str) -> String {
    // Case 1: Slide starts with <h1> tag (standard markdown header)
    if let Some(h1_start) = slide.find("<h1>") {
        if let Some(h1_end) = slide.find("</h1>") {
            // Get title text without h1 tags
            let title = &slide[h1_start + 4..h1_end];

            // Get content after the h1 tag
            let body = &slide[h1_end + 5..];

            // Return title and body - preserve HTML
            if body.trim().is_empty() {
                title.to_string()
            } else {
                format!("{}\n{}", title, body.trim())
            }
        } else {
            slide.to_string()
        }
    }
    // Case 2: Slide contains <p>#Text</p> format (no space after #)
    else if let Some(p_start) = slide.find("<p>") {
        if let Some(p_end) = slide.find("</p>") {
            let p_content = &slide[p_start + 3..p_end];

            // Check if content starts with # and remove it
            let clean_content = p_content.strip_prefix('#').unwrap_or(p_content);

            // Extract any remaining content after this paragraph
            let rest = &slide[p_end + 4..];

            // Return content preserving HTML
            if rest.trim().is_empty() {
                clean_content.to_string()
            } else {
                format!("{}\n{}", clean_content, rest.trim())
            }
        } else {
            slide.to_string()
        }
    }
    // Default: use the slide content as is
    else {
        slide.to_string()
    }
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
