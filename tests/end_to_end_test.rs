use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run_command(args: &[&str]) -> Output {
    Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .output()
        .expect("Failed to execute command")
}

fn count_files_with_pattern(dir: &Path, pattern: &str) -> usize {
    let glob_pattern = format!("{}/{}", dir.to_string_lossy(), pattern);
    glob::glob(&glob_pattern)
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .count()
}

// Create some CSS content
fn create_test_css() -> (String, String) {
    let css_content = r#"
        body {
            font-family: Arial, sans-serif;
            color: #333;
        }
        h1 {
            color: #0066cc;
            border-bottom: 2px solid #ccc;
        }
        .slides > div {
            margin-bottom: 20px;
            border: 1px solid #eee;
            padding: 20px;
        }
    "#;

    let path = "./test-style.css".to_string();

    (path, css_content.to_string())
}

// Create test JavaScript for navigation
fn create_test_js() -> (String, String) {
    let js_content = r#"
        document.addEventListener('DOMContentLoaded', function() {
            const slides = document.querySelectorAll('.slides > div');
            let currentSlide = 0;
            
            // Hide all slides except the first one
            for (let i = 1; i < slides.length; i++) {
                slides[i].style.display = 'none';
            }
            
            // Handle arrow key navigation
            document.addEventListener('keydown', function(e) {
                if (e.key === 'ArrowRight' && currentSlide < slides.length - 1) {
                    slides[currentSlide].style.display = 'none';
                    currentSlide++;
                    slides[currentSlide].style.display = 'block';
                } else if (e.key === 'ArrowLeft' && currentSlide > 0) {
                    slides[currentSlide].style.display = 'none';
                    currentSlide--;
                    slides[currentSlide].style.display = 'block';
                }
            });
        });
    "#;

    let path = "./test-nav.js".to_string();

    (path, js_content.to_string())
}

#[test]
#[ignore] // Ignore by default as it requires a headless browser
fn test_full_pipeline() {
    // Set up a logger to see progress
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_dir = temp_dir.path();

    // Create sample markdown file
    let markdown_path = base_dir.join("test.md");
    let html_path = base_dir.join("test.html");
    let slides_dir = base_dir.join("slides");
    let pptx_path = base_dir.join("presentation.pptx");

    // Create slides directory
    fs::create_dir(&slides_dir).expect("Failed to create slides directory");

    // Create a rich markdown file with multiple slides and various content features
    let markdown_content = r#"# Big Slides Presentation

## A sample presentation generated with big-slides

---

# Slide 1: Introduction

* Bullet point 1
* Bullet point 2
* Bullet point 3

---

# Slide 2: Code Example

```rust
fn main() {
    println!("Hello, world!");
}
```

---

# Slide 3: Tables

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

---

# Slide 4: Mixed Content

## Subheading

1. Numbered item 1
2. Numbered item 2

> This is a blockquote
"#;

    fs::write(&markdown_path, markdown_content).expect("Failed to write markdown file");

    // Create CSS and JS files
    let (css_path, css_content) = create_test_css();
    let css_file = base_dir.join(&css_path);
    fs::write(&css_file, css_content).expect("Failed to write CSS file");

    let (js_path, js_content) = create_test_js();
    let js_file = base_dir.join(&js_path);
    fs::write(&js_file, js_content).expect("Failed to write JS file");

    // STEP 1: Generate HTML from markdown with CSS and JS
    println!("STEP 1: Generating HTML from markdown with styling");
    let html_output = run_command(&[
        "generate-html",
        "-i",
        markdown_path.to_str().unwrap(),
        "-o",
        html_path.to_str().unwrap(),
        "--css",
        css_file.to_str().unwrap(),
        "--js",
        js_file.to_str().unwrap(),
    ]);

    assert!(
        html_output.status.success(),
        "generate-html command failed: {:?}",
        String::from_utf8_lossy(&html_output.stderr)
    );
    assert!(html_path.exists(), "HTML file was not created");

    // Verify HTML content includes our CSS and JS
    let html_content = fs::read_to_string(&html_path).expect("Failed to read HTML file");
    assert!(
        html_content.contains("<style>"),
        "CSS was not embedded in HTML"
    );
    assert!(
        html_content.contains("<script>"),
        "JavaScript was not embedded in HTML"
    );
    assert!(
        html_content.contains("font-family: Arial"),
        "CSS content is incorrect"
    );
    assert!(
        html_content.contains("document.addEventListener"),
        "JS content is incorrect"
    );

    // STEP 2: Generate slides from HTML with custom dimensions
    println!("STEP 2: Generating slides from HTML");
    let slides_output = run_command(&[
        "generate-slides",
        "-i",
        html_path.to_str().unwrap(),
        "-o",
        slides_dir.to_str().unwrap(),
        "--width",
        "1024", // Widescreen format
        "--height",
        "768",
        "--base-name",
        "presentation", // Custom base name
        "--format",
        "png",
    ]);

    assert!(
        slides_output.status.success(),
        "generate-slides command failed: {:?}",
        String::from_utf8_lossy(&slides_output.stderr)
    );

    // Check that slide images were created with correct naming
    let slide_count = count_files_with_pattern(&slides_dir, "presentation_*.png");
    assert!(slide_count > 0, "No slide images were created");
    println!("Generated {} slide images", slide_count);

    // Verify we have the expected number of slides
    assert_eq!(slide_count, 5, "Expected 5 slides but got {}", slide_count);

    // STEP 3: Generate PPTX from slides with custom title
    println!("STEP 3: Generating PPTX from slides");
    let pptx_output = run_command(&[
        "generate-pptx",
        "-i",
        slides_dir.to_str().unwrap(),
        "-o",
        pptx_path.to_str().unwrap(),
        "--title",
        "Big Slides Demo",
        "--pattern",
        "presentation_*.png", // Match our custom naming
    ]);

    assert!(
        pptx_output.status.success(),
        "generate-pptx command failed: {:?}",
        String::from_utf8_lossy(&pptx_output.stderr)
    );
    assert!(pptx_path.exists(), "PPTX file was not created");

    // Validation of PPTX
    let metadata = fs::metadata(&pptx_path).expect("Failed to get PPTX metadata");
    assert!(metadata.len() > 0, "PPTX file is empty");

    // Try to verify PPTX is a valid ZIP file (minimal structure check)
    let file = fs::File::open(&pptx_path).expect("Failed to open PPTX file");
    let mut archive = zip::ZipArchive::new(file).expect("Failed to read PPTX as ZIP");

    // Check for essential PPTX structure files
    let essential_files = vec![
        "[Content_Types].xml",
        "ppt/presentation.xml",
        "ppt/slides/slide1.xml",
    ];

    for file in essential_files {
        assert!(
            archive.by_name(file).is_ok(),
            "Missing essential file in PPTX: {}",
            file
        );
    }

    println!("End-to-end test completed successfully!");
}

#[test]
#[ignore] // Ignore as it requires headless browser
fn test_pipeline_with_empty_content() {
    // Test with minimal content to check edge cases
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base_dir = temp_dir.path();

    // Create sample markdown file with minimal content
    let markdown_path = base_dir.join("empty.md");
    let html_path = base_dir.join("empty.html");
    let slides_dir = base_dir.join("empty_slides");
    let pptx_path = base_dir.join("empty.pptx");

    fs::create_dir(&slides_dir).expect("Failed to create slides directory");

    // Very minimal, single slide
    let markdown_content = "# Empty Test";
    fs::write(&markdown_path, markdown_content).expect("Failed to write markdown file");

    // Generate HTML
    let html_output = run_command(&[
        "generate-html",
        "-i",
        markdown_path.to_str().unwrap(),
        "-o",
        html_path.to_str().unwrap(),
    ]);

    assert!(html_output.status.success(), "generate-html command failed");

    // Generate slides
    let slides_output = run_command(&[
        "generate-slides",
        "-i",
        html_path.to_str().unwrap(),
        "-o",
        slides_dir.to_str().unwrap(),
    ]);

    assert!(
        slides_output.status.success(),
        "generate-slides command failed"
    );

    // Generate PPTX
    let pptx_output = run_command(&[
        "generate-pptx",
        "-i",
        slides_dir.to_str().unwrap(),
        "-o",
        pptx_path.to_str().unwrap(),
    ]);

    assert!(pptx_output.status.success(), "generate-pptx command failed");
    assert!(pptx_path.exists(), "PPTX file was not created");

    println!("Empty content pipeline test completed successfully!");
}
