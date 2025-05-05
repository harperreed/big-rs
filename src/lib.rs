// ABOUTME: Library module for the big-slides program.
// ABOUTME: Contains core functionality for generating HTML, slides, and PPTX files.

use anyhow::{Context, Result};
use comrak::{markdown_to_html, ComrakOptions};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum BigError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to fetch remote resource: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("Invalid resource path: {0}")]
    InvalidResourcePath(String),
}

#[derive(Debug, Clone)]
pub struct ResourceFile {
    path: String,
    is_remote: bool,
}

impl ResourceFile {
    pub fn new(path: &str) -> Self {
        let is_remote = path.starts_with("http://") || path.starts_with("https://");
        Self {
            path: path.to_string(),
            is_remote,
        }
    }

    pub fn content(&self) -> Result<String, BigError> {
        if self.is_remote {
            let content = reqwest::blocking::get(&self.path)
                .map_err(|e| BigError::FetchError(e))?
                .text()
                .map_err(|e| BigError::FetchError(e))?;
            Ok(content)
        } else {
            let content = fs::read_to_string(&self.path)
                .map_err(|e| BigError::FileReadError(e))?;
            Ok(content)
        }
    }

    pub fn tag(&self, tag_type: &str) -> String {
        match tag_type {
            "css" => {
                if self.is_remote {
                    format!(r#"<link rel="stylesheet" href="{}">"#, self.path)
                } else {
                    let content = self.content().unwrap_or_default();
                    format!(r#"<style>{}</style>"#, content)
                }
            }
            "js" => {
                if self.is_remote {
                    format!(r#"<script src="{}"></script>"#, self.path)
                } else {
                    let content = self.content().unwrap_or_default();
                    format!(r#"<script>{}</script>"#, content)
                }
            }
            _ => String::new(),
        }
    }
}

/// Generate HTML from a markdown file
pub fn generate_html(
    markdown_path: &Path,
    css_files: &[ResourceFile],
    js_files: &[ResourceFile],
) -> Result<String> {
    // Read markdown content
    let markdown_content = fs::read_to_string(markdown_path)
        .with_context(|| format!("Failed to read markdown file: {:?}", markdown_path))?;

    // Convert markdown to HTML
    let options = ComrakOptions::default();
    let html_content = markdown_to_html(&markdown_content, &options);

    // Build the full HTML document
    let mut html_doc = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html_doc.push_str("<meta charset=\"UTF-8\">\n");
    html_doc.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html_doc.push_str("<title>Presentation</title>\n");

    // Add CSS
    for css in css_files {
        html_doc.push_str(&css.tag("css"));
        html_doc.push('\n');
    }

    html_doc.push_str("</head>\n<body>\n");

    // Wrap content in div for slides
    html_doc.push_str("<div class=\"slides\">\n");
    html_doc.push_str(&html_content);
    html_doc.push_str("\n</div>\n");

    // Add JavaScript
    for js in js_files {
        html_doc.push_str(&js.tag("js"));
        html_doc.push('\n');
    }

    html_doc.push_str("</body>\n</html>");

    Ok(html_doc)
}

/// Generate slides (images) from HTML
pub fn generate_slides(
    html_path: &Path,
    output_dir: &Path,
    base_name: &str,
    format: &str,
    width: u32,
    height: u32,
) -> Result<Vec<PathBuf>> {
    use headless_chrome::{Browser, LaunchOptions};
    use log::info;
    use std::time::Duration;
    
    info!("Launching headless browser");
    
    // Launch headless Chrome browser
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .window_size(Some((width, height)))
            .headless(true)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Chrome: {}", e))?,
    )
    .map_err(|e| anyhow::anyhow!("Failed to launch Chrome: {}", e))?;
    
    // Get the HTML file URL
    let html_path_abs = fs::canonicalize(html_path)
        .with_context(|| format!("Failed to get absolute path for {:?}", html_path))?;
    let url = format!("file://{}", html_path_abs.to_string_lossy());
    
    info!("Opening page at URL: {}", url);
    
    // Create a new tab and navigate to the HTML file
    let tab = browser.new_tab()
        .map_err(|e| anyhow::anyhow!("Failed to create new tab: {}", e))?;
    
    tab.navigate_to(&url)
        .map_err(|e| anyhow::anyhow!("Failed to navigate to HTML: {}", e))?;
    
    // Wait for page to load
    tab.wait_until_navigated()
        .map_err(|e| anyhow::anyhow!("Navigation failed: {}", e))?;
    
    // Take screenshot of the first slide
    info!("Taking screenshot of slide 1");
    let screenshot_data = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    )
    .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;
    
    // Save screenshot to file
    let output_file = output_dir.join(format!("{}_0001.{}", base_name, format));
    fs::write(&output_file, &screenshot_data)
        .with_context(|| format!("Failed to write screenshot to {:?}", output_file))?;
    
    info!("Screenshot saved to {:?}", output_file);
    let mut output_files = vec![output_file];
    
    // Try to detect more slides
    if let Ok(total_slides) = tab.evaluate("document.querySelectorAll('.slides > *').length", false) {
        // The RemoteObject contains the slides count as a number
        let count_str = format!("{:?}", total_slides.value);
        // Parse the count from debug string representation
        let count = count_str.trim_matches(|c| c == '"' || c == ' ')
            .parse::<i64>()
            .unwrap_or(1);
        info!("Detected {} slides", count);
        
        if count > 1 {
            // Iterate through rest of slides
            for i in 1..count {
                // Press right arrow key to advance to next slide
                tab.press_key("ArrowRight")
                    .map_err(|e| anyhow::anyhow!("Failed to press right arrow key: {}", e))?;
                
                // Wait a bit for transition
                std::thread::sleep(Duration::from_millis(500));
                
                // Take screenshot
                info!("Taking screenshot of slide {}", i + 1);
                let screenshot_data = tab.capture_screenshot(
                    headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                    None,
                    None,
                    true
                )
                .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;
                
                // Save screenshot
                let output_file = output_dir.join(format!("{}_{:04}.{}", base_name, i + 1, format));
                fs::write(&output_file, &screenshot_data)
                    .with_context(|| format!("Failed to write screenshot to {:?}", output_file))?;
                
                info!("Screenshot saved to {:?}", output_file);
                output_files.push(output_file);
            }
        }
    }
    
    Ok(output_files)
}

/// Generate PPTX from slides
pub fn generate_pptx(
    slides_dir: &Path,
    output_file: &Path,
    pattern: &str,
    title: &str,
) -> Result<()> {
    use image::io::Reader as ImageReader;
    use log::info;
    use std::io::Write;
    use uuid::Uuid;
    use zip::{ZipWriter, write::FileOptions};
    
    info!("Generating PPTX from slides in {:?}", slides_dir);
    
    // Collect all slide image files matching the pattern
    let mut slide_paths = Vec::new();
    for entry in glob::glob(&format!("{}/{}", slides_dir.to_string_lossy(), pattern))? {
        if let Ok(path) = entry {
            slide_paths.push(path);
        }
    }
    
    // Sort slide paths to ensure they're in the correct order
    slide_paths.sort();
    
    info!("Found {} slide images", slide_paths.len());
    if slide_paths.is_empty() {
        return Err(anyhow::anyhow!("No slide images found in directory"));
    }
    
    // Create a new PPTX file
    let file = fs::File::create(output_file)?;
    let mut zip = ZipWriter::new(file);
    
    // Create a unique presentation ID (unused but kept for reference)
    let _presentation_id = Uuid::new_v4().to_string();
    
    // Add [Content_Types].xml
    zip.start_file("[Content_Types].xml", FileOptions::default())?;
    let content_types = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="jpeg" ContentType="image/jpeg"/>
    <Default Extension="png" ContentType="image/png"/>
    <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
    <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
    <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
    {slides}
</Types>"#, 
        slides = slide_paths.iter().enumerate().map(|(i, _)| {
            format!(r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#, i + 1)
        }).collect::<Vec<String>>().join("\n")
    );
    zip.write_all(content_types.as_bytes())?;
    
    // Add _rels/.rels
    fs::create_dir_all(output_file.parent().unwrap().join("_rels"))?;
    zip.start_file("_rels/.rels", FileOptions::default())?;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;
    zip.write_all(rels.as_bytes())?;
    
    // Add docProps/app.xml
    fs::create_dir_all(output_file.parent().unwrap().join("docProps"))?;
    zip.start_file("docProps/app.xml", FileOptions::default())?;
    let app_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Application>big-slides</Application>
    <Slides>{}</Slides>
</Properties>"#, slide_paths.len());
    zip.write_all(app_xml.as_bytes())?;
    
    // Add docProps/core.xml
    zip.start_file("docProps/core.xml", FileOptions::default())?;
    let core_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <dc:title>{}</dc:title>
    <dc:creator>big-slides</dc:creator>
    <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
    <cp:revision>1</cp:revision>
</cp:coreProperties>"#, 
        title,
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
    zip.write_all(core_xml.as_bytes())?;
    
    // Create ppt directory
    fs::create_dir_all(output_file.parent().unwrap().join("ppt"))?;
    
    // Add ppt/_rels/presentation.xml.rels
    fs::create_dir_all(output_file.parent().unwrap().join("ppt/_rels"))?;
    zip.start_file("ppt/_rels/presentation.xml.rels", FileOptions::default())?;
    
    let mut pres_rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#);
    
    // Add relationship for each slide
    for (i, _) in slide_paths.iter().enumerate() {
        pres_rels.push_str(&format!(
            r#"    <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            i + 1, i + 1
        ));
        pres_rels.push('\n');
    }
    
    pres_rels.push_str("</Relationships>");
    zip.write_all(pres_rels.as_bytes())?;
    
    // Add ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", FileOptions::default())?;
    let presentation_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:sldIdLst>
{slide_ids}
    </p:sldIdLst>
    <p:sldSz cx="9144000" cy="5143500" type="screen4x3"/>
    <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#, 
        slide_ids = slide_paths.iter().enumerate().map(|(i, _)| {
            format!(r#"        <p:sldId id="{}" r:id="rId{}"/>"#, 256 + i, i + 1)
        }).collect::<Vec<String>>().join("\n")
    );
    zip.write_all(presentation_xml.as_bytes())?;
    
    // Create directories for slides and media
    fs::create_dir_all(output_file.parent().unwrap().join("ppt/slides"))?;
    fs::create_dir_all(output_file.parent().unwrap().join("ppt/slides/_rels"))?;
    fs::create_dir_all(output_file.parent().unwrap().join("ppt/media"))?;
    
    // Process each slide
    for (i, slide_path) in slide_paths.iter().enumerate() {
        let slide_num = i + 1;
        info!("Processing slide {}: {:?}", slide_num, slide_path);
        
        // Add the image to the media directory
        let image_ext = slide_path.extension().unwrap_or_default().to_string_lossy().to_string();
        let image_name = format!("image{}.{}", slide_num, image_ext);
        let image_data = fs::read(slide_path)?;
        
        zip.start_file(format!("ppt/media/{}", image_name), FileOptions::default())?;
        zip.write_all(&image_data)?;
        
        // Create slide_rels file
        zip.start_file(format!("ppt/slides/_rels/slide{}.xml.rels", slide_num), FileOptions::default())?;
        let slide_rels = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{}"/>
</Relationships>"#, image_name);
        zip.write_all(slide_rels.as_bytes())?;
        
        // Get image dimensions (unused but kept for potential future use)
        let img = ImageReader::open(slide_path)?.decode()?;
        let (_width, _height) = (img.width() as i32, img.height() as i32);
        
        // Create slide file
        zip.start_file(format!("ppt/slides/slide{}.xml", slide_num), FileOptions::default())?;
        let slide_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:nvGrpSpPr>
                <p:cNvPr id="1" name=""/>
                <p:cNvGrpSpPr/>
                <p:nvPr/>
            </p:nvGrpSpPr>
            <p:grpSpPr>
                <a:xfrm>
                    <a:off x="0" y="0"/>
                    <a:ext cx="0" cy="0"/>
                    <a:chOff x="0" y="0"/>
                    <a:chExt cx="0" cy="0"/>
                </a:xfrm>
            </p:grpSpPr>
            <p:pic>
                <p:nvPicPr>
                    <p:cNvPr id="2" name="Image"/>
                    <p:cNvPicPr>
                        <a:picLocks noChangeAspect="1"/>
                    </p:cNvPicPr>
                    <p:nvPr/>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId1"/>
                    <a:stretch>
                        <a:fillRect/>
                    </a:stretch>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="0" y="0"/>
                        <a:ext cx="9144000" cy="5143500"/>
                    </a:xfrm>
                    <a:prstGeom prst="rect">
                        <a:avLst/>
                    </a:prstGeom>
                </p:spPr>
            </p:pic>
        </p:spTree>
    </p:cSld>
    <p:clrMapOvr>
        <a:masterClrMapping/>
    </p:clrMapOvr>
</p:sld>"#);
        zip.write_all(slide_xml.as_bytes())?;
    }
    
    // Finalize the ZIP file
    zip.finish()?;
    
    info!("PPTX file created at {:?}", output_file);
    Ok(())
}