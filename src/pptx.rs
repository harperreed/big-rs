// ABOUTME: PPTX generation module for the big-slides application
// ABOUTME: Creates PowerPoint presentations from slide images

use crate::errors::{BigError, Result};
use chrono;
use glob;
use image::io::Reader as ImageReader;
use log::{info, warn};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::{ZipWriter, write::FileOptions};

/// Configuration for PPTX generation
pub struct PptxConfig {
    pub title: String,
    pub pattern: String,
    pub aspect_ratio: String, // "16:9" or "4:3"
}

impl Default for PptxConfig {
    fn default() -> Self {
        Self {
            title: "Presentation".to_string(),
            pattern: "*.png".to_string(),
            aspect_ratio: "16:9".to_string(),
        }
    }
}

/// Generate a PPTX presentation from slide images
pub fn generate_pptx(slides_dir: &Path, output_file: &Path, config: &PptxConfig) -> Result<()> {
    info!("Generating PPTX from slides in {:?}", slides_dir);

    // Validate input directory exists
    if !slides_dir.exists() || !slides_dir.is_dir() {
        return Err(BigError::PathNotFoundError(slides_dir.to_path_buf()));
    }

    // Ensure parent directory for output file exists
    if let Some(parent) = output_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(BigError::FileReadError)?;
        }
    }

    // Collect all slide image files matching the pattern
    let mut slide_paths = Vec::new();
    let glob_pattern = format!("{}/{}", slides_dir.to_string_lossy(), config.pattern);

    for entry in (glob::glob(&glob_pattern)
        .map_err(|e| BigError::PptxError(format!("Invalid glob pattern: {}", e)))?)
    .flatten()
    {
        slide_paths.push(entry);
    }

    // Sort slide paths to ensure they're in the correct order
    slide_paths.sort();

    info!("Found {} slide images", slide_paths.len());
    if slide_paths.is_empty() {
        return Err(BigError::NoSlidesFoundError(glob_pattern));
    }

    // Create a new PPTX file
    let file = fs::File::create(output_file).map_err(BigError::FileReadError)?;
    let mut zip = ZipWriter::new(file);

    // Set slide dimensions based on aspect ratio
    let (cx, cy) = match config.aspect_ratio.as_str() {
        "16:9" => (9144000, 5143500), // 16:9 ratio
        "4:3" => (9144000, 6858000),  // 4:3 ratio
        _ => {
            warn!(
                "Unsupported aspect ratio: {}. Using 16:9 instead.",
                config.aspect_ratio
            );
            (9144000, 5143500) // Default to 16:9
        }
    };

    // Add [Content_Types].xml
    info!("Creating PPTX structure: [Content_Types].xml");
    zip.start_file("[Content_Types].xml", FileOptions::default())?;
    let content_types = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="jpeg" ContentType="image/jpeg"/>
    <Default Extension="jpg" ContentType="image/jpeg"/>
    <Default Extension="png" ContentType="image/png"/>
    <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
    <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
    <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
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
    info!("Creating PPTX structure: _rels/.rels");
    zip.start_file("_rels/.rels", FileOptions::default())?;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;
    zip.write_all(rels.as_bytes())?;

    // Add docProps/app.xml
    info!("Creating PPTX structure: docProps/app.xml");
    zip.start_file("docProps/app.xml", FileOptions::default())?;
    let app_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Application>big-slides</Application>
    <Slides>{}</Slides>
</Properties>"#,
        slide_paths.len()
    );
    zip.write_all(app_xml.as_bytes())?;

    // Add docProps/core.xml
    info!("Creating PPTX structure: docProps/core.xml");
    zip.start_file("docProps/core.xml", FileOptions::default())?;
    let core_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <dc:title>{}</dc:title>
    <dc:creator>big-slides</dc:creator>
    <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
    <cp:revision>1</cp:revision>
</cp:coreProperties>"#,
        config.title,
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
    zip.write_all(core_xml.as_bytes())?;

    // Add ppt/_rels/presentation.xml.rels
    info!("Creating PPTX structure: ppt/_rels/presentation.xml.rels");
    zip.start_file("ppt/_rels/presentation.xml.rels", FileOptions::default())?;

    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
    );

    // Add relationship for slide master (must be before slides)
    pres_rels.push_str(
        r#"    <Relationship Id="rIdSM" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
"#,
    );

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
    info!("Creating PPTX structure: ppt/presentation.xml");
    zip.start_file("ppt/presentation.xml", FileOptions::default())?;
    
    // Determine correct slide size type
    let slide_size_type = match config.aspect_ratio.as_str() {
        "16:9" => "screen16x9",
        "4:3" => "screen4x3",
        _ => "screen16x9" // Default to 16:9
    };
    
    let presentation_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" 
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
    <p:sldMasterIdLst>
        <p:sldMasterId id="2147483648" r:id="rIdSM"/>
    </p:sldMasterIdLst>
    <p:sldIdLst>
{slide_ids}
    </p:sldIdLst>
    <p:sldSz cx="{cx}" cy="{cy}" type="{slide_size_type}"/>
    <p:notesSz cx="6858000" cy="9144000"/>
    <p:defaultTextStyle>
        <a:defPPr/>
        <a:lvl1pPr marL="0" algn="l" defTabSz="914400" rtl="0" eaLnBrk="1" latinLnBrk="0" hangingPunct="1">
            <a:defRPr sz="1800" kern="1200">
                <a:solidFill>
                    <a:schemeClr val="tx1"/>
                </a:solidFill>
                <a:latin typeface="+mn-lt"/>
                <a:ea typeface="+mn-ea"/>
                <a:cs typeface="+mn-cs"/>
            </a:defRPr>
        </a:lvl1pPr>
    </p:defaultTextStyle>
</p:presentation>"#,
        slide_ids = slide_paths
            .iter()
            .enumerate()
            .map(|(i, _)| { format!(r#"        <p:sldId id="{}" r:id="rId{}"/>"#, 256 + i, i + 1) })
            .collect::<Vec<String>>()
            .join("\n"),
        cx = cx,
        cy = cy,
        slide_size_type = slide_size_type
    );
    zip.write_all(presentation_xml.as_bytes())?;
    
    // Add a basic slide master
    info!("Creating slide master: ppt/slideMasters/slideMaster1.xml");
    zip.start_file("ppt/slideMasters/slideMaster1.xml", FileOptions::default())?;
    let slide_master_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" 
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:bg>
            <p:bgRef idx="1001">
                <a:schemeClr val="bg1"/>
            </p:bgRef>
        </p:bg>
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
        </p:spTree>
    </p:cSld>
    <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#
    );
    zip.write_all(slide_master_xml.as_bytes())?;
    
    // Add slide master relationships
    info!("Creating slide master relationships: ppt/slideMasters/_rels/slideMaster1.xml.rels");
    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", FileOptions::default())?;
    let slide_master_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;
    zip.write_all(slide_master_rels.as_bytes())?;
    
    // Add a basic slide layout
    info!("Creating slide layout: ppt/slideLayouts/slideLayout1.xml");
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", FileOptions::default())?;
    let slide_layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" 
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank">
    <p:cSld name="Blank">
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
        </p:spTree>
    </p:cSld>
    <p:clrMapOvr>
        <a:masterClrMapping/>
    </p:clrMapOvr>
</p:sldLayout>"#;
    zip.write_all(slide_layout_xml.as_bytes())?;
    
    // Add slide layout relationships
    info!("Creating slide layout relationships: ppt/slideLayouts/_rels/slideLayout1.xml.rels");
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", FileOptions::default())?;
    let slide_layout_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;
    zip.write_all(slide_layout_rels.as_bytes())?;

    // Process each slide
    for (i, slide_path) in slide_paths.iter().enumerate() {
        let slide_num = i + 1;
        info!("Processing slide {}: {:?}", slide_num, slide_path);

        // Add the image to the media directory
        let image_ext = slide_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let image_name = format!("image{}.{}", slide_num, image_ext);

        // Read image data
        let image_data = match fs::read(slide_path) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to read image file {:?}: {}", slide_path, e);
                continue; // Skip this slide but continue with others
            }
        };

        // Add image to media directory
        info!("Adding image to PPTX: ppt/media/{}", image_name);
        zip.start_file(format!("ppt/media/{}", image_name), FileOptions::default())?;
        zip.write_all(&image_data)?;

        // Create slide_rels file
        info!(
            "Creating slide relationships: ppt/slides/_rels/slide{}.xml.rels",
            slide_num
        );
        zip.start_file(
            format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
            FileOptions::default(),
        )?;
        let slide_rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{}"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#,
            image_name
        );
        zip.write_all(slide_rels.as_bytes())?;

        // Verify image can be read and decoded (for validation)
        match ImageReader::open(slide_path) {
            Ok(reader) => match reader.decode() {
                Ok(_) => {
                    // Image is valid, continue processing
                }
                Err(e) => {
                    warn!("Failed to decode image {:?}: {}", slide_path, e);
                    continue; // Skip this slide but continue with others
                }
            },
            Err(e) => {
                warn!("Failed to open image {:?}: {}", slide_path, e);
                continue;
            }
        };

        // Create slide file
        info!("Creating slide XML: ppt/slides/slide{}.xml", slide_num);
        zip.start_file(
            format!("ppt/slides/slide{}.xml", slide_num),
            FileOptions::default(),
        )?;
        let slide_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" 
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" 
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
    <p:cSld name="Slide {slide_num}">
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
                    <p:cNvPr id="2" name="Slide Image {slide_num}"/>
                    <p:cNvPicPr>
                        <a:picLocks noChangeAspect="1"/>
                    </p:cNvPicPr>
                    <p:nvPr/>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId1">
                        <a:extLst>
                            <a:ext uri="{{28A0092B-C50C-407E-A947-70E740481C1C}}">
                                <a14:useLocalDpi xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main" val="0"/>
                            </a:ext>
                        </a:extLst>
                    </a:blip>
                    <a:stretch>
                        <a:fillRect/>
                    </a:stretch>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="0" y="0"/>
                        <a:ext cx="{cx}" cy="{cy}"/>
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
</p:sld>"#,
            cx = cx,
            cy = cy,
            slide_num = slide_num
        );
        zip.write_all(slide_xml.as_bytes())?;
    }

    // Finalize the ZIP file
    info!("Finalizing PPTX file");
    zip.finish()?;

    info!("PPTX file created at {:?}", output_file);
    Ok(())
}

/// Find slide images that match a pattern in a directory
pub fn find_slide_images(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let glob_pattern = format!("{}/{}", dir.to_string_lossy(), pattern);
    let mut paths = Vec::new();

    for entry in (glob::glob(&glob_pattern)
        .map_err(|e| BigError::PptxError(format!("Invalid glob pattern: {}", e)))?)
    .flatten()
    {
        paths.push(entry);
    }

    paths.sort();

    if paths.is_empty() {
        return Err(BigError::NoSlidesFoundError(glob_pattern));
    }

    Ok(paths)
}
