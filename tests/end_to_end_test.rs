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
    
    // Create a simple markdown file with multiple slides
    let markdown_content = r#"# Slide 1
    
This is the first slide.

---

# Slide 2

This is the second slide.

---

# Slide 3

This is the third slide.
    "#;
    
    fs::write(&markdown_path, markdown_content).expect("Failed to write markdown file");
    
    // Step 1: Generate HTML from markdown
    println!("STEP 1: Generating HTML from markdown");
    let html_output = run_command(&[
        "generate-html",
        "-i", markdown_path.to_str().unwrap(),
        "-o", html_path.to_str().unwrap(),
    ]);
    
    assert!(html_output.status.success(), 
        "generate-html command failed: {:?}", String::from_utf8_lossy(&html_output.stderr));
    assert!(html_path.exists(), "HTML file was not created");
    
    // Step 2: Generate slides from HTML
    println!("STEP 2: Generating slides from HTML");
    let slides_output = run_command(&[
        "generate-slides",
        "-i", html_path.to_str().unwrap(),
        "-o", slides_dir.to_str().unwrap(),
        "--width", "800",
        "--height", "600",
    ]);
    
    assert!(slides_output.status.success(), 
        "generate-slides command failed: {:?}", String::from_utf8_lossy(&slides_output.stderr));
    
    // Check that slide images were created
    let slide_files: Vec<_> = fs::read_dir(&slides_dir)
        .expect("Failed to read slides directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy().to_string();
            file_name.starts_with("slide_") && file_name.ends_with(".png")
        })
        .collect();
    
    assert!(!slide_files.is_empty(), "No slide images were created");
    
    // Step 3: Generate PPTX from slides
    println!("STEP 3: Generating PPTX from slides");
    let pptx_output = run_command(&[
        "generate-pptx",
        "-i", slides_dir.to_str().unwrap(),
        "-o", pptx_path.to_str().unwrap(),
        "--title", "Test Presentation",
    ]);
    
    assert!(pptx_output.status.success(), 
        "generate-pptx command failed: {:?}", String::from_utf8_lossy(&pptx_output.stderr));
    assert!(pptx_path.exists(), "PPTX file was not created");
    
    // Basic validation of PPTX
    let metadata = fs::metadata(&pptx_path).expect("Failed to get PPTX metadata");
    assert!(metadata.len() > 0, "PPTX file is empty");
    
    println!("End-to-end test completed successfully!");
}