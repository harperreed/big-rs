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
fn test_generate_pptx_basic() {
    // Create a temporary directory for slides
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let slide_dir = temp_dir.path();
    
    // Create some test slide images (small colored squares for quick test)
    let slide1_path = slide_dir.join("slide_0001.png");
    let slide2_path = slide_dir.join("slide_0002.png");
    
    // Create two small colored images
    let red_img = image::ImageBuffer::from_fn(100, 100, |_, _| image::Rgb([255u8, 0u8, 0u8]));
    let blue_img = image::ImageBuffer::from_fn(100, 100, |_, _| image::Rgb([0u8, 0u8, 255u8]));
    
    // Save the images
    red_img.save(&slide1_path).expect("Failed to save red image");
    blue_img.save(&slide2_path).expect("Failed to save blue image");
    
    // Output PPTX file
    let output_path = temp_dir.path().join("output.pptx");
    
    // Generate the PPTX
    let result = generate_pptx(
        slide_dir,
        &output_path,
        "slide_*.png",
        "Test Presentation"
    );
    
    assert!(result.is_ok(), "Failed to generate PPTX: {:?}", result.err());
    assert!(output_path.exists(), "PPTX file was not created");
    
    // Verify basic ZIP structure with the zip library
    let file = fs::File::open(&output_path).expect("Failed to open PPTX file");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read PPTX as ZIP");
    
    // Check for some essential files
    assert!(archive.by_name("[Content_Types].xml").is_ok(), "Missing [Content_Types].xml");
    assert!(archive.by_name("ppt/presentation.xml").is_ok(), "Missing presentation.xml");
    assert!(archive.by_name("ppt/slides/slide1.xml").is_ok(), "Missing slide1.xml");
    assert!(archive.by_name("ppt/slides/slide2.xml").is_ok(), "Missing slide2.xml");
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