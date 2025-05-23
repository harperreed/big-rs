use super::*;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

fn create_temp_markdown_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    file
}

fn create_temp_resource_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    file
}

fn create_temp_html_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    file
}

#[test]
fn test_generate_html_basic() {
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Check that the HTML includes the markdown content without h1 tags
    // The div should directly contain the text "Test Slide" followed by the content
    assert!(html.contains("<div>Test Slide") && html.contains("This is a test slide."));

    // Check that the HTML has the proper structure
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html lang=\"en\">"));
    // No longer expecting slides wrapper div, as we now directly output slides as divs under body
    assert!(html.contains("<body>\n<div>"));
}

#[test]
fn test_generate_html_with_local_css() {
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let css_content = "body { font-family: Arial; }";
    let css_file = create_temp_resource_file(css_content);

    let css_resource = ResourceFile::new(css_file.path().to_str().unwrap());

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[css_resource],
        &[],
        true, // embed resources
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

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[js_resource],
        true, // embed resources
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

    let tag = resource.tag("css", false).unwrap(); // link, not embed
    assert_eq!(
        tag,
        r#"<link rel="stylesheet" href="https://example.com/style.css">"#
    );

    let resource = ResourceFile::new("https://example.com/script.js");
    let tag = resource.tag("js", false).unwrap(); // link, not embed
    assert_eq!(
        tag,
        r#"<script src="https://example.com/script.js"></script>"#
    );
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
    red_img
        .save(&slide1_path)
        .expect("Failed to save red image");
    blue_img
        .save(&slide2_path)
        .expect("Failed to save blue image");

    // Output PPTX file
    let output_path = temp_dir.path().join("output.pptx");

    // Create PPTX configuration
    let pptx_config = PptxConfig {
        title: "Test Presentation".to_string(),
        pattern: "slide_*.png".to_string(),
        aspect_ratio: "16:9".to_string(),
    };

    // Generate the PPTX
    let result = pptx::generate_pptx(slide_dir, &output_path, &pptx_config);

    assert!(
        result.is_ok(),
        "Failed to generate PPTX: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "PPTX file was not created");

    // Verify basic ZIP structure with the zip library
    let file = fs::File::open(&output_path).expect("Failed to open PPTX file");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read PPTX as ZIP");

    // Check for some essential files
    assert!(
        archive.by_name("[Content_Types].xml").is_ok(),
        "Missing [Content_Types].xml"
    );
    assert!(
        archive.by_name("ppt/presentation.xml").is_ok(),
        "Missing presentation.xml"
    );
    assert!(
        archive.by_name("ppt/slides/slide1.xml").is_ok(),
        "Missing slide1.xml"
    );
    assert!(
        archive.by_name("ppt/slides/slide2.xml").is_ok(),
        "Missing slide2.xml"
    );
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
    <div>Hello Slide</div>
</body>
</html>"#;

    let html_file = create_temp_html_file(html_content);
    let output_dir = TempDir::new().expect("Failed to create temp dir");

    // Create render configuration with custom values for test
    let render_config = RenderConfig {
        width: 800,
        height: 600,
        format: "png".to_string(),
        base_name: "test".to_string(),
        timeout_ms: 30000,
        browser_path: None,
    };

    // Generate slides
    let result = render::generate_slides(
        &html_file.path().to_path_buf(),
        &output_dir.path().to_path_buf(),
        &render_config,
    );

    assert!(
        result.is_ok(),
        "Failed to generate slides: {:?}",
        result.err()
    );

    let output_files = result.unwrap();
    assert_eq!(output_files.len(), 1, "Should generate 1 slide");

    // Check that the file exists
    assert!(output_files[0].exists(), "Output file should exist");
}

#[test]
fn test_default_css_js_config_values() {
    // Verify that Config's default CSS and JS values match the expected URLs
    let config = config::Config::default();

    assert_eq!(
        config.default_css, "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.css",
        "Default CSS URL should match expected value"
    );
    assert_eq!(
        config.default_js, "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.js",
        "Default JS URL should match expected value"
    );

    // Create a simple markdown file
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    // Create ResourceFile from default values
    let css_resource = ResourceFile::new(&config.default_css);
    let js_resource = ResourceFile::new(&config.default_js);

    // Generate HTML with the default resources explicitly provided
    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[css_resource],
        &[js_resource],
        false, // Don't embed resources to make link checking easier
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Verify CSS is included correctly
    let expected_css_link = format!(r#"<link rel="stylesheet" href="{}">"#, config.default_css);
    assert!(
        html.contains(&expected_css_link),
        "HTML should contain default CSS link: {}",
        expected_css_link
    );

    // Verify JS is included correctly
    let expected_js_link = format!(r#"<script src="{}">"#, config.default_js);
    assert!(
        html.contains(&expected_js_link),
        "HTML should contain default JS script tag"
    );
}

#[test]
fn test_header_format_with_space() {
    // Test the "# Header" format (with space)
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Check that the HTML includes the markdown content without h1 tags
    assert!(
        html.contains("<div>Test Slide") && html.contains("This is a test slide."),
        "HTML should contain slide content without the h1 tags"
    );
}

#[test]
fn test_header_format_without_space() {
    // Test the "#Header" format (no space)
    let markdown_content = "#Test Slide\n\nThis is a test slide.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // The content should be properly processed to handle the no-space format
    // and should NOT contain the # character in the output
    assert!(
        html.contains("<div>Test Slide") && html.contains("This is a test slide."),
        "HTML should contain slide content without the # character"
    );
    assert!(
        !html.contains("<div>#Test Slide"),
        "HTML should not contain the # character in the output"
    );
}

#[test]
fn test_header_format_complex_case() {
    // Test more complex header cases with mixed content
    let markdown_content = "# First Slide\n\nStandard slide with space after #.\n\n\
                           #Second Slide\n\nSlide with no space after #.\n\n\
                           #Special Characters: !@#$%^&*()_+\n\nSlide with special characters.\n\n\
                           #   Extra Spaces\n\nSlide with multiple spaces after #.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Check that all slides are properly processed
    assert!(
        html.contains("<div>First Slide"),
        "First slide should be included with proper format"
    );
    assert!(
        html.contains("<div>Second Slide"),
        "Second slide should be included with proper format"
    );
    assert!(
        html.contains("<div>Special Characters: !@#$%^&amp;*()_+"),
        "Slide with special characters should be included properly"
    );
    assert!(
        html.contains("<div>Extra Spaces"),
        "Slide with extra spaces should be included properly"
    );

    // Ensure no slides contain the # character in their title
    assert!(
        !html.contains("<div>#"),
        "HTML should not contain the # character in any slide title"
    );
}

#[test]
fn test_header_format_multiple_slides() {
    // Test multiple slides with mixed header formats
    let markdown_content =
        "# Slide One\n\nContent for slide one.\n\n#Slide Two\n\nContent for slide two.";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Check for first slide content without h1 tags
    assert!(
        html.contains("<div>Slide One") && html.contains("Content for slide one."),
        "HTML should contain first slide content correctly"
    );

    // Check for second slide content without h1 tags and without # character
    assert!(
        html.contains("<div>Slide Two") && html.contains("Content for slide two."),
        "HTML should contain second slide content correctly without # character"
    );

    // Make sure we don't have the # character in any of the output
    assert!(
        !html.contains("<div>#Slide"),
        "HTML should not contain the # character in any slide"
    );
}

#[test]
fn test_slide_with_html_content() {
    // Test a slide with HTML content that should be preserved
    let markdown_content = "# Slide With HTML\n\n<div class=\"special\">This is <em>formatted</em> content</div>\n\n<ul>\n<li>Item 1</li>\n<li>Item 2</li>\n</ul>";
    let markdown_file = create_temp_markdown_file(markdown_content);

    let result = html::generate_html_without_reload(
        &markdown_file.path().to_path_buf(),
        &[],
        &[],
        true, // embed resources
    );

    assert!(result.is_ok());
    let html = result.unwrap();

    // Check that the HTML tags are preserved in the output
    assert!(
        html.contains(
            "<div>Slide With HTML\n<div class=\"special\">This is <em>formatted</em> content</div>"
        ),
        "HTML should preserve div and em tags"
    );
    assert!(
        html.contains("<ul>\n<li>Item 1</li>\n<li>Item 2</li>\n</ul>"),
        "HTML should preserve ul and li tags"
    );
}
