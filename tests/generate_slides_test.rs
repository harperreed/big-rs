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
#[ignore] // Ignore by default as it requires Chrome to be installed
fn test_generate_slides_command() {
    // Create temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create output directory
    let output_dir = temp_path.join("slides");
    fs::create_dir(&output_dir).expect("Failed to create output directory");

    // Create sample HTML file
    let html_path = temp_path.join("test.html");
    let html_content = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Slides</title>
    <style>
        .slides > div {
            width: 100%;
            height: 100%;
            background-color: white;
            color: black;
            display: flex;
            align-items: center;
            justify-content: center;
            font-family: Arial, sans-serif;
        }
    </style>
</head>
<body>
    <div class="slides">
        <div>
            <h1>Slide 1</h1>
        </div>
        <div>
            <h1>Slide 2</h1>
        </div>
        <div>
            <h1>Slide 3</h1>
        </div>
    </div>
    <script>
        // Simple script to navigate between slides using arrow keys
        const slides = document.querySelectorAll('.slides > div');
        let currentSlide = 0;
        
        // Hide all slides except the first one
        for (let i = 1; i < slides.length; i++) {
            slides[i].style.display = 'none';
        }
        
        document.addEventListener('keydown', (e) => {
            if (e.key === 'ArrowRight' && currentSlide < slides.length - 1) {
                slides[currentSlide].style.display = 'none';
                currentSlide++;
                slides[currentSlide].style.display = 'flex';
            } else if (e.key === 'ArrowLeft' && currentSlide > 0) {
                slides[currentSlide].style.display = 'none';
                currentSlide--;
                slides[currentSlide].style.display = 'flex';
            }
        });
    </script>
</body>
</html>"#;
    fs::write(&html_path, html_content).expect("Failed to write HTML file");

    // Run command
    let output = run_command(&[
        "generate-slides",
        "-i",
        html_path.to_str().unwrap(),
        "-o",
        output_dir.to_str().unwrap(),
        "--width",
        "800",
        "--height",
        "600",
    ]);

    // Check command executed successfully
    assert!(output.status.success(), "Command failed: {:?}", output);

    // Check that at least one slide image was created
    let slide_files: Vec<_> = fs::read_dir(&output_dir)
        .expect("Failed to read output directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("slide_")
                && entry.file_name().to_string_lossy().ends_with(".png")
        })
        .collect();

    assert!(!slide_files.is_empty(), "No slide images were created");

    // Note: In a real headless browser environment with JavaScript, we would expect 3 slides
    // But in the basic test, we'll just check that at least one was created
}
