use super::*;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

fn create_temp_markdown_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes()).expect("Failed to write to temp file");
    file
}

fn create_temp_resource_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes()).expect("Failed to write to temp file");
    file
}

fn create_temp_html_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes()).expect("Failed to write to temp file");
    file
}

#[test]
fn test_generate_html_basic() {
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);
    
    let result = generate_html(
        &markdown_file.path().to_path_buf(),
        &[],
        &[]
    );
    
    assert!(result.is_ok());
    let html = result.unwrap();
    
    // Check that the HTML includes the markdown content
    assert!(html.contains("<h1>Test Slide</h1>"));
    assert!(html.contains("<p>This is a test slide.</p>"));
    
    // Check that the HTML has the proper structure
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html lang=\"en\">"));
    assert!(html.contains("<div class=\"slides\">"));
}

#[test]
fn test_generate_html_with_local_css() {
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);
    
    let css_content = "body { font-family: Arial; }";
    let css_file = create_temp_resource_file(css_content);
    
    let css_resource = ResourceFile::new(css_file.path().to_str().unwrap());
    
    let result = generate_html(
        &markdown_file.path().to_path_buf(),
        &[css_resource],
        &[]
    );
    
    assert!(result.is_ok());
    let html = result.unwrap();
    
    // Check that the CSS is embedded
    assert!(html.contains("<style>body { font-family: Arial; }</style>"));
}

#[test]
fn test_generate_html_with_local_js() {
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);
    
    let js_content = "function testFunction() { return true; }";
    let js_file = create_temp_resource_file(js_content);
    
    let js_resource = ResourceFile::new(js_file.path().to_str().unwrap());
    
    let result = generate_html(
        &markdown_file.path().to_path_buf(),
        &[],
        &[js_resource]
    );
    
    assert!(result.is_ok());
    let html = result.unwrap();
    
    // Check that the JS is embedded
    assert!(html.contains("<script>function testFunction() { return true; }</script>"));
}

#[test]
fn test_resource_file_remote() {
    let resource = ResourceFile::new("https://example.com/style.css");
    assert!(resource.is_remote);
    
    let tag = resource.tag("css");
    assert_eq!(tag, r#"<link rel="stylesheet" href="https://example.com/style.css">"#);
    
    let resource = ResourceFile::new("https://example.com/script.js");
    let tag = resource.tag("js");
    assert_eq!(tag, r#"<script src="https://example.com/script.js"></script>"#);
}

#[test]
#[ignore] // Ignore by default as it requires Chrome to be installed
fn test_generate_slides_basic() {
    // Create a simple HTML file with one slide
    let html_content = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Slide</title>
    <style>
        .slides > div {
            width: 100%;
            height: 100%;
            background-color: white;
            color: black;
            display: flex;
            align-items: center;
            justify-content: center;
        }
    </style>
</head>
<body>
    <div class="slides">
        <div>
            <h1>Hello Slide</h1>
        </div>
    </div>
</body>
</html>"#;
    
    let html_file = create_temp_html_file(html_content);
    let output_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Generate slides
    let result = generate_slides(
        &html_file.path().to_path_buf(),
        &output_dir.path().to_path_buf(),
        "test",
        "png",
        800,
        600,
    );
    
    assert!(result.is_ok(), "Failed to generate slides: {:?}", result.err());
    
    let output_files = result.unwrap();
    assert_eq!(output_files.len(), 1, "Should generate 1 slide");
    
    // Check that the file exists
    assert!(output_files[0].exists(), "Output file should exist");
}