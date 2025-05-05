use std::fs;
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

#[test]
fn test_generate_html_command() {
    // Create temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create sample markdown file
    let markdown_path = temp_path.join("test.md");
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    fs::write(&markdown_path, markdown_content).expect("Failed to write markdown file");

    // Create sample CSS file
    let css_path = temp_path.join("test.css");
    let css_content = "body { font-family: Arial; }";
    fs::write(&css_path, css_content).expect("Failed to write CSS file");

    // Output HTML path
    let output_path = temp_path.join("output.html");

    // Run command
    let output = run_command(&[
        "generate-html",
        "-i",
        markdown_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "--css",
        css_path.to_str().unwrap(),
    ]);

    // Check command executed successfully
    assert!(output.status.success(), "Command failed: {:?}", output);

    // Check output file exists
    assert!(output_path.exists(), "Output file was not created");

    // Read output file
    let html_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    // Verify output file content
    assert!(
        html_content.contains("<h1>Test Slide</h1>"),
        "Missing markdown content"
    );
    assert!(
        html_content.contains("<style>body { font-family: Arial; }</style>"),
        "Missing CSS"
    );
}

#[test]
fn test_default_css_js_inclusion() {
    // Create temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create sample markdown file
    let markdown_path = temp_path.join("test.md");
    let markdown_content = "# Test Slide\n\nThis is a test slide.";
    fs::write(&markdown_path, markdown_content).expect("Failed to write markdown file");

    // Output HTML path
    let output_path = temp_path.join("output.html");

    // Run command without specifying CSS or JS files to test defaults
    let output = run_command(&[
        "generate-html",
        "-i",
        markdown_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "--mode",
        "link", // Use link mode to make it easier to check for URLs
    ]);

    // Check command executed successfully
    assert!(output.status.success(), "Command failed: {:?}", output);

    // Check output file exists
    assert!(output_path.exists(), "Output file was not created");

    // Read output file
    let html_content = fs::read_to_string(&output_path).expect("Failed to read output file");

    // Verify output file content
    assert!(
        html_content.contains("<h1>Test Slide</h1>"),
        "Missing markdown content"
    );

    // Verify default CSS is included
    let default_css = "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.css";
    assert!(
        html_content.contains(&format!(
            r#"<link rel="stylesheet" href="{}">"#,
            default_css
        )),
        "Missing default CSS link"
    );

    // Verify default JS is included
    let default_js = "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.js";
    assert!(
        html_content.contains(&format!(r#"<script src="{}">"#, default_js)),
        "Missing default JS script tag"
    );
}
