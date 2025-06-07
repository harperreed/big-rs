use image::{ImageBuffer, Rgb};
use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;
use zip::ZipArchive;

fn run_command(args: &[&str]) -> Output {
    Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .output()
        .expect("Failed to execute command")
}

#[test]
fn test_generate_pptx_command() {
    // Create temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let slide_dir = temp_dir.path().join("slides");
    fs::create_dir(&slide_dir).expect("Failed to create slides directory");

    // Create some test slide images
    let slide1_path = slide_dir.join("slide_0001.png");
    let slide2_path = slide_dir.join("slide_0002.png");

    // Create two simple colored images
    let red_img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
    let blue_img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([0u8, 0u8, 255u8]));

    // Save the images
    red_img
        .save(&slide1_path)
        .expect("Failed to save red image");
    blue_img
        .save(&slide2_path)
        .expect("Failed to save blue image");

    // Output PPTX file path
    let output_path = temp_dir.path().join("output.pptx");

    // Run command
    let output = run_command(&[
        "generate-pptx",
        "-i",
        slide_dir.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "--pattern",
        "slide_*.png",
        "--title",
        "Test Presentation",
    ]);

    // Check command executed successfully
    assert!(output.status.success(), "Command failed: {:?}", output);

    // Check output file exists
    assert!(output_path.exists(), "PPTX file was not created");

    // Check file size is not zero (very basic check)
    let metadata = fs::metadata(&output_path).expect("Failed to get file metadata");
    assert!(metadata.len() > 0, "PPTX file is empty");

    // Verify slide files within the PPTX archive
    let file = fs::File::open(&output_path).expect("Failed to open PPTX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read PPTX as ZIP");
    let slide_files: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .map(|f| f.name().to_string())
        })
        .filter(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
        .collect();

    assert_eq!(slide_files.len(), 2, "Expected exactly two slide XML files");
    assert!(slide_files.contains(&"ppt/slides/slide1.xml".to_string()));
    assert!(slide_files.contains(&"ppt/slides/slide2.xml".to_string()));
}
