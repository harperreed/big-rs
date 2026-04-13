// ABOUTME: PPTX generation module for the big-slides application
// ABOUTME: Creates PowerPoint presentations from slide images

use crate::errors::{BigError, Result};
use chrono;
use glob;
use image::io::Reader as ImageReader;
use log::{info, warn};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};

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

/// Write an XML declaration (<?xml version="1.0" ...?>) to the writer
fn write_xml_decl(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
    writer
        .write_event(Event::Decl(BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            Some("yes"),
        )))
        .map_err(|e| BigError::PptxError(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Write an empty (self-closing) XML element with attributes
fn write_empty_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tag: &str,
    attrs: &[(&str, &str)],
) -> Result<()> {
    let mut elem = BytesStart::new(tag);
    for (key, value) in attrs {
        elem.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Empty(elem))
        .map_err(|e| BigError::PptxError(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Write an XML start tag with attributes
fn write_start(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tag: &str,
    attrs: &[(&str, &str)],
) -> Result<()> {
    let mut elem = BytesStart::new(tag);
    for (key, value) in attrs {
        elem.push_attribute((*key, *value));
    }
    writer
        .write_event(Event::Start(elem))
        .map_err(|e| BigError::PptxError(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Write an XML end tag
fn write_end(writer: &mut Writer<Cursor<Vec<u8>>>, tag: &str) -> Result<()> {
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| BigError::PptxError(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Write text content inside the current element
fn write_text(writer: &mut Writer<Cursor<Vec<u8>>>, text: &str) -> Result<()> {
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|e| BigError::PptxError(format!("XML write error: {}", e)))?;
    Ok(())
}

/// Create an indented XML writer backed by an in-memory buffer
fn new_xml_writer() -> Writer<Cursor<Vec<u8>>> {
    Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4)
}

/// Extract the finished XML bytes from the writer
fn finish_xml(writer: Writer<Cursor<Vec<u8>>>) -> Vec<u8> {
    writer.into_inner().into_inner()
}

/// Write a simple text element: <tag>text</tag>
fn write_text_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tag: &str,
    attrs: &[(&str, &str)],
    text: &str,
) -> Result<()> {
    write_start(writer, tag, attrs)?;
    write_text(writer, text)?;
    write_end(writer, tag)?;
    Ok(())
}

/// Build the [Content_Types].xml document
fn build_content_types_xml(slide_count: usize) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Types",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/content-types",
        )],
    )?;

    // Default content types
    write_empty_element(
        &mut writer,
        "Default",
        &[("Extension", "xml"), ("ContentType", "application/xml")],
    )?;
    write_empty_element(
        &mut writer,
        "Default",
        &[
            ("Extension", "rels"),
            (
                "ContentType",
                "application/vnd.openxmlformats-package.relationships+xml",
            ),
        ],
    )?;
    write_empty_element(
        &mut writer,
        "Default",
        &[("Extension", "jpeg"), ("ContentType", "image/jpeg")],
    )?;
    write_empty_element(
        &mut writer,
        "Default",
        &[("Extension", "jpg"), ("ContentType", "image/jpeg")],
    )?;
    write_empty_element(
        &mut writer,
        "Default",
        &[("Extension", "png"), ("ContentType", "image/png")],
    )?;

    // Override content types for fixed parts
    write_empty_element(&mut writer, "Override", &[("PartName", "/ppt/presentation.xml"), ("ContentType", "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml")])?;
    write_empty_element(
        &mut writer,
        "Override",
        &[
            ("PartName", "/ppt/slideMasters/slideMaster1.xml"),
            (
                "ContentType",
                "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml",
            ),
        ],
    )?;
    write_empty_element(
        &mut writer,
        "Override",
        &[
            ("PartName", "/ppt/slideLayouts/slideLayout1.xml"),
            (
                "ContentType",
                "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml",
            ),
        ],
    )?;
    write_empty_element(
        &mut writer,
        "Override",
        &[
            ("PartName", "/docProps/core.xml"),
            (
                "ContentType",
                "application/vnd.openxmlformats-package.core-properties+xml",
            ),
        ],
    )?;
    write_empty_element(
        &mut writer,
        "Override",
        &[
            ("PartName", "/docProps/app.xml"),
            (
                "ContentType",
                "application/vnd.openxmlformats-officedocument.extended-properties+xml",
            ),
        ],
    )?;

    // Override content types for each slide
    for i in 1..=slide_count {
        let part_name = format!("/ppt/slides/slide{}.xml", i);
        write_empty_element(
            &mut writer,
            "Override",
            &[
                ("PartName", part_name.as_str()),
                (
                    "ContentType",
                    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
                ),
            ],
        )?;
    }

    write_end(&mut writer, "Types")?;
    Ok(finish_xml(writer))
}

/// Build the _rels/.rels document
fn build_rels_xml() -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Relationships",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )],
    )?;

    write_empty_element(&mut writer, "Relationship", &[
        ("Id", "rId1"),
        ("Type", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"),
        ("Target", "ppt/presentation.xml"),
    ])?;
    write_empty_element(&mut writer, "Relationship", &[
        ("Id", "rId2"),
        ("Type", "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties"),
        ("Target", "docProps/core.xml"),
    ])?;
    write_empty_element(&mut writer, "Relationship", &[
        ("Id", "rId3"),
        ("Type", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties"),
        ("Target", "docProps/app.xml"),
    ])?;

    write_end(&mut writer, "Relationships")?;
    Ok(finish_xml(writer))
}

/// Build the docProps/app.xml document
fn build_app_xml(slide_count: usize) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Properties",
        &[
            (
                "xmlns",
                "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
            ),
            (
                "xmlns:vt",
                "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes",
            ),
        ],
    )?;

    write_text_element(&mut writer, "Application", &[], "big-slides")?;
    write_text_element(&mut writer, "Slides", &[], &slide_count.to_string())?;

    write_end(&mut writer, "Properties")?;
    Ok(finish_xml(writer))
}

/// Build the docProps/core.xml document
fn build_core_xml(title: &str) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "cp:coreProperties",
        &[
            (
                "xmlns:cp",
                "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
            ),
            ("xmlns:dc", "http://purl.org/dc/elements/1.1/"),
            ("xmlns:dcterms", "http://purl.org/dc/terms/"),
            ("xmlns:dcmitype", "http://purl.org/dc/dcmitype/"),
            ("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"),
        ],
    )?;

    write_text_element(&mut writer, "dc:title", &[], title)?;
    write_text_element(&mut writer, "dc:creator", &[], "big-slides")?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    write_text_element(
        &mut writer,
        "dcterms:created",
        &[("xsi:type", "dcterms:W3CDTF")],
        &timestamp,
    )?;
    write_text_element(&mut writer, "cp:revision", &[], "1")?;

    write_end(&mut writer, "cp:coreProperties")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/_rels/presentation.xml.rels document
fn build_presentation_rels_xml(slide_count: usize) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Relationships",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )],
    )?;

    // Slide master relationship (must be before slides)
    write_empty_element(
        &mut writer,
        "Relationship",
        &[
            ("Id", "rIdSM"),
            (
                "Type",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster",
            ),
            ("Target", "slideMasters/slideMaster1.xml"),
        ],
    )?;

    // Per-slide relationships
    for i in 1..=slide_count {
        let id = format!("rId{}", i);
        let target = format!("slides/slide{}.xml", i);
        write_empty_element(
            &mut writer,
            "Relationship",
            &[
                ("Id", id.as_str()),
                (
                    "Type",
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide",
                ),
                ("Target", target.as_str()),
            ],
        )?;
    }

    write_end(&mut writer, "Relationships")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/presentation.xml document
fn build_presentation_xml(
    slide_count: usize,
    cx: i64,
    cy: i64,
    slide_size_type: &str,
) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "p:presentation",
        &[
            (
                "xmlns:a",
                "http://schemas.openxmlformats.org/drawingml/2006/main",
            ),
            (
                "xmlns:r",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            ),
            (
                "xmlns:p",
                "http://schemas.openxmlformats.org/presentationml/2006/main",
            ),
            (
                "xmlns:mc",
                "http://schemas.openxmlformats.org/markup-compatibility/2006",
            ),
        ],
    )?;

    // Slide master ID list
    write_start(&mut writer, "p:sldMasterIdLst", &[])?;
    write_empty_element(
        &mut writer,
        "p:sldMasterId",
        &[("id", "2147483648"), ("r:id", "rIdSM")],
    )?;
    write_end(&mut writer, "p:sldMasterIdLst")?;

    // Slide ID list
    write_start(&mut writer, "p:sldIdLst", &[])?;
    for i in 0..slide_count {
        let id = (256 + i).to_string();
        let r_id = format!("rId{}", i + 1);
        write_empty_element(
            &mut writer,
            "p:sldId",
            &[("id", id.as_str()), ("r:id", r_id.as_str())],
        )?;
    }
    write_end(&mut writer, "p:sldIdLst")?;

    // Slide size
    let cx_str = cx.to_string();
    let cy_str = cy.to_string();
    write_empty_element(
        &mut writer,
        "p:sldSz",
        &[
            ("cx", cx_str.as_str()),
            ("cy", cy_str.as_str()),
            ("type", slide_size_type),
        ],
    )?;

    // Notes size
    write_empty_element(
        &mut writer,
        "p:notesSz",
        &[("cx", "6858000"), ("cy", "9144000")],
    )?;

    // Default text style
    write_start(&mut writer, "p:defaultTextStyle", &[])?;
    write_empty_element(&mut writer, "a:defPPr", &[])?;

    write_start(
        &mut writer,
        "a:lvl1pPr",
        &[
            ("marL", "0"),
            ("algn", "l"),
            ("defTabSz", "914400"),
            ("rtl", "0"),
            ("eaLnBrk", "1"),
            ("latinLnBrk", "0"),
            ("hangingPunct", "1"),
        ],
    )?;

    write_start(&mut writer, "a:defRPr", &[("sz", "1800"), ("kern", "1200")])?;
    write_start(&mut writer, "a:solidFill", &[])?;
    write_empty_element(&mut writer, "a:schemeClr", &[("val", "tx1")])?;
    write_end(&mut writer, "a:solidFill")?;
    write_empty_element(&mut writer, "a:latin", &[("typeface", "+mn-lt")])?;
    write_empty_element(&mut writer, "a:ea", &[("typeface", "+mn-ea")])?;
    write_empty_element(&mut writer, "a:cs", &[("typeface", "+mn-cs")])?;
    write_end(&mut writer, "a:defRPr")?;

    write_end(&mut writer, "a:lvl1pPr")?;
    write_end(&mut writer, "p:defaultTextStyle")?;

    write_end(&mut writer, "p:presentation")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/slideMasters/slideMaster1.xml document
fn build_slide_master_xml() -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "p:sldMaster",
        &[
            (
                "xmlns:a",
                "http://schemas.openxmlformats.org/drawingml/2006/main",
            ),
            (
                "xmlns:r",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            ),
            (
                "xmlns:p",
                "http://schemas.openxmlformats.org/presentationml/2006/main",
            ),
        ],
    )?;

    write_start(&mut writer, "p:cSld", &[])?;

    // Background
    write_start(&mut writer, "p:bg", &[])?;
    write_start(&mut writer, "p:bgRef", &[("idx", "1001")])?;
    write_empty_element(&mut writer, "a:schemeClr", &[("val", "bg1")])?;
    write_end(&mut writer, "p:bgRef")?;
    write_end(&mut writer, "p:bg")?;

    // Shape tree
    write_shape_tree(&mut writer)?;

    write_end(&mut writer, "p:cSld")?;

    // Color map
    write_empty_element(
        &mut writer,
        "p:clrMap",
        &[
            ("bg1", "lt1"),
            ("tx1", "dk1"),
            ("bg2", "lt2"),
            ("tx2", "dk2"),
            ("accent1", "accent1"),
            ("accent2", "accent2"),
            ("accent3", "accent3"),
            ("accent4", "accent4"),
            ("accent5", "accent5"),
            ("accent6", "accent6"),
            ("hlink", "hlink"),
            ("folHlink", "folHlink"),
        ],
    )?;

    write_end(&mut writer, "p:sldMaster")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/slideMasters/_rels/slideMaster1.xml.rels document
fn build_slide_master_rels_xml() -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Relationships",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )],
    )?;
    write_empty_element(
        &mut writer,
        "Relationship",
        &[
            ("Id", "rId1"),
            (
                "Type",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout",
            ),
            ("Target", "../slideLayouts/slideLayout1.xml"),
        ],
    )?;
    write_end(&mut writer, "Relationships")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/slideLayouts/slideLayout1.xml document
fn build_slide_layout_xml() -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "p:sldLayout",
        &[
            (
                "xmlns:a",
                "http://schemas.openxmlformats.org/drawingml/2006/main",
            ),
            (
                "xmlns:r",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            ),
            (
                "xmlns:p",
                "http://schemas.openxmlformats.org/presentationml/2006/main",
            ),
            ("type", "blank"),
        ],
    )?;

    write_start(&mut writer, "p:cSld", &[("name", "Blank")])?;
    write_shape_tree(&mut writer)?;
    write_end(&mut writer, "p:cSld")?;

    // Color map override
    write_start(&mut writer, "p:clrMapOvr", &[])?;
    write_empty_element(&mut writer, "a:masterClrMapping", &[])?;
    write_end(&mut writer, "p:clrMapOvr")?;

    write_end(&mut writer, "p:sldLayout")?;
    Ok(finish_xml(writer))
}

/// Build the ppt/slideLayouts/_rels/slideLayout1.xml.rels document
fn build_slide_layout_rels_xml() -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    write_start(
        &mut writer,
        "Relationships",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )],
    )?;
    write_empty_element(
        &mut writer,
        "Relationship",
        &[
            ("Id", "rId1"),
            (
                "Type",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster",
            ),
            ("Target", "../slideMasters/slideMaster1.xml"),
        ],
    )?;
    write_end(&mut writer, "Relationships")?;
    Ok(finish_xml(writer))
}

/// Build the per-slide relationships XML
fn build_slide_rels_xml(image_name: &str) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    let image_target = format!("../media/{}", image_name);

    write_start(
        &mut writer,
        "Relationships",
        &[(
            "xmlns",
            "http://schemas.openxmlformats.org/package/2006/relationships",
        )],
    )?;
    write_empty_element(
        &mut writer,
        "Relationship",
        &[
            ("Id", "rId1"),
            (
                "Type",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
            ),
            ("Target", image_target.as_str()),
        ],
    )?;
    write_empty_element(
        &mut writer,
        "Relationship",
        &[
            ("Id", "rId2"),
            (
                "Type",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout",
            ),
            ("Target", "../slideLayouts/slideLayout1.xml"),
        ],
    )?;
    write_end(&mut writer, "Relationships")?;
    Ok(finish_xml(writer))
}

/// Build the per-slide XML document
fn build_slide_xml(slide_num: usize, cx: i64, cy: i64) -> Result<Vec<u8>> {
    let mut writer = new_xml_writer();
    write_xml_decl(&mut writer)?;

    let slide_name = format!("Slide {}", slide_num);
    let image_desc = format!("Slide Image {}", slide_num);

    write_start(
        &mut writer,
        "p:sld",
        &[
            (
                "xmlns:a",
                "http://schemas.openxmlformats.org/drawingml/2006/main",
            ),
            (
                "xmlns:r",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            ),
            (
                "xmlns:p",
                "http://schemas.openxmlformats.org/presentationml/2006/main",
            ),
            (
                "xmlns:mc",
                "http://schemas.openxmlformats.org/markup-compatibility/2006",
            ),
        ],
    )?;

    write_start(&mut writer, "p:cSld", &[("name", slide_name.as_str())])?;
    write_start(&mut writer, "p:spTree", &[])?;

    // Group shape properties (standard boilerplate)
    write_start(&mut writer, "p:nvGrpSpPr", &[])?;
    write_empty_element(&mut writer, "p:cNvPr", &[("id", "1"), ("name", "")])?;
    write_empty_element(&mut writer, "p:cNvGrpSpPr", &[])?;
    write_empty_element(&mut writer, "p:nvPr", &[])?;
    write_end(&mut writer, "p:nvGrpSpPr")?;

    write_start(&mut writer, "p:grpSpPr", &[])?;
    write_start(&mut writer, "a:xfrm", &[])?;
    write_empty_element(&mut writer, "a:off", &[("x", "0"), ("y", "0")])?;
    write_empty_element(&mut writer, "a:ext", &[("cx", "0"), ("cy", "0")])?;
    write_empty_element(&mut writer, "a:chOff", &[("x", "0"), ("y", "0")])?;
    write_empty_element(&mut writer, "a:chExt", &[("cx", "0"), ("cy", "0")])?;
    write_end(&mut writer, "a:xfrm")?;
    write_end(&mut writer, "p:grpSpPr")?;

    // Picture element
    write_start(&mut writer, "p:pic", &[])?;

    // Non-visual picture properties
    write_start(&mut writer, "p:nvPicPr", &[])?;
    write_empty_element(
        &mut writer,
        "p:cNvPr",
        &[("id", "2"), ("name", image_desc.as_str())],
    )?;
    write_start(&mut writer, "p:cNvPicPr", &[])?;
    write_empty_element(&mut writer, "a:picLocks", &[("noChangeAspect", "1")])?;
    write_end(&mut writer, "p:cNvPicPr")?;
    write_empty_element(&mut writer, "p:nvPr", &[])?;
    write_end(&mut writer, "p:nvPicPr")?;

    // Blip fill (image reference)
    write_start(&mut writer, "p:blipFill", &[])?;
    write_start(&mut writer, "a:blip", &[("r:embed", "rId1")])?;
    write_start(&mut writer, "a:extLst", &[])?;
    write_start(
        &mut writer,
        "a:ext",
        &[("uri", "{28A0092B-C50C-407E-A947-70E740481C1C}")],
    )?;
    write_empty_element(
        &mut writer,
        "a14:useLocalDpi",
        &[
            (
                "xmlns:a14",
                "http://schemas.microsoft.com/office/drawing/2010/main",
            ),
            ("val", "0"),
        ],
    )?;
    write_end(&mut writer, "a:ext")?;
    write_end(&mut writer, "a:extLst")?;
    write_end(&mut writer, "a:blip")?;
    write_start(&mut writer, "a:stretch", &[])?;
    write_empty_element(&mut writer, "a:fillRect", &[])?;
    write_end(&mut writer, "a:stretch")?;
    write_end(&mut writer, "p:blipFill")?;

    // Shape properties (position and size)
    let cx_str = cx.to_string();
    let cy_str = cy.to_string();
    write_start(&mut writer, "p:spPr", &[])?;
    write_start(&mut writer, "a:xfrm", &[])?;
    write_empty_element(&mut writer, "a:off", &[("x", "0"), ("y", "0")])?;
    write_empty_element(
        &mut writer,
        "a:ext",
        &[("cx", cx_str.as_str()), ("cy", cy_str.as_str())],
    )?;
    write_end(&mut writer, "a:xfrm")?;
    write_start(&mut writer, "a:prstGeom", &[("prst", "rect")])?;
    write_empty_element(&mut writer, "a:avLst", &[])?;
    write_end(&mut writer, "a:prstGeom")?;
    write_end(&mut writer, "p:spPr")?;

    write_end(&mut writer, "p:pic")?;
    write_end(&mut writer, "p:spTree")?;
    write_end(&mut writer, "p:cSld")?;

    // Color map override
    write_start(&mut writer, "p:clrMapOvr", &[])?;
    write_empty_element(&mut writer, "a:masterClrMapping", &[])?;
    write_end(&mut writer, "p:clrMapOvr")?;

    write_end(&mut writer, "p:sld")?;
    Ok(finish_xml(writer))
}

/// Write the standard empty shape tree used in master/layout slides
fn write_shape_tree(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
    write_start(writer, "p:spTree", &[])?;

    write_start(writer, "p:nvGrpSpPr", &[])?;
    write_empty_element(writer, "p:cNvPr", &[("id", "1"), ("name", "")])?;
    write_empty_element(writer, "p:cNvGrpSpPr", &[])?;
    write_empty_element(writer, "p:nvPr", &[])?;
    write_end(writer, "p:nvGrpSpPr")?;

    write_start(writer, "p:grpSpPr", &[])?;
    write_start(writer, "a:xfrm", &[])?;
    write_empty_element(writer, "a:off", &[("x", "0"), ("y", "0")])?;
    write_empty_element(writer, "a:ext", &[("cx", "0"), ("cy", "0")])?;
    write_empty_element(writer, "a:chOff", &[("x", "0"), ("y", "0")])?;
    write_empty_element(writer, "a:chExt", &[("cx", "0"), ("cy", "0")])?;
    write_end(writer, "a:xfrm")?;
    write_end(writer, "p:grpSpPr")?;

    write_end(writer, "p:spTree")?;
    Ok(())
}

/// Generate a PPTX presentation from slide images
pub fn generate_pptx(slides_dir: &Path, output_file: &Path, config: &PptxConfig) -> Result<()> {
    info!("Generating PPTX from slides in {:?}", slides_dir);

    // Validate input directory exists
    if !slides_dir.exists() || !slides_dir.is_dir() {
        return Err(BigError::PathNotFoundError(slides_dir.to_path_buf()));
    }

    // Ensure parent directory for output file exists
    #[allow(clippy::collapsible_if)]
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

    let slide_count = slide_paths.len();

    // Add [Content_Types].xml
    info!("Creating PPTX structure: [Content_Types].xml");
    zip.start_file("[Content_Types].xml", FileOptions::default())?;
    zip.write_all(&build_content_types_xml(slide_count)?)?;

    // Add _rels/.rels
    info!("Creating PPTX structure: _rels/.rels");
    zip.start_file("_rels/.rels", FileOptions::default())?;
    zip.write_all(&build_rels_xml()?)?;

    // Add docProps/app.xml
    info!("Creating PPTX structure: docProps/app.xml");
    zip.start_file("docProps/app.xml", FileOptions::default())?;
    zip.write_all(&build_app_xml(slide_count)?)?;

    // Add docProps/core.xml
    info!("Creating PPTX structure: docProps/core.xml");
    zip.start_file("docProps/core.xml", FileOptions::default())?;
    zip.write_all(&build_core_xml(&config.title)?)?;

    // Add ppt/_rels/presentation.xml.rels
    info!("Creating PPTX structure: ppt/_rels/presentation.xml.rels");
    zip.start_file("ppt/_rels/presentation.xml.rels", FileOptions::default())?;
    zip.write_all(&build_presentation_rels_xml(slide_count)?)?;

    // Add ppt/presentation.xml
    info!("Creating PPTX structure: ppt/presentation.xml");
    zip.start_file("ppt/presentation.xml", FileOptions::default())?;

    // Determine correct slide size type
    let slide_size_type = match config.aspect_ratio.as_str() {
        "16:9" => "screen16x9",
        "4:3" => "screen4x3",
        _ => "screen16x9", // Default to 16:9
    };

    zip.write_all(&build_presentation_xml(
        slide_count,
        cx,
        cy,
        slide_size_type,
    )?)?;

    // Add a basic slide master
    info!("Creating slide master: ppt/slideMasters/slideMaster1.xml");
    zip.start_file("ppt/slideMasters/slideMaster1.xml", FileOptions::default())?;
    zip.write_all(&build_slide_master_xml()?)?;

    // Add slide master relationships
    info!("Creating slide master relationships: ppt/slideMasters/_rels/slideMaster1.xml.rels");
    zip.start_file(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        FileOptions::default(),
    )?;
    zip.write_all(&build_slide_master_rels_xml()?)?;

    // Add a basic slide layout
    info!("Creating slide layout: ppt/slideLayouts/slideLayout1.xml");
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", FileOptions::default())?;
    zip.write_all(&build_slide_layout_xml()?)?;

    // Add slide layout relationships
    info!("Creating slide layout relationships: ppt/slideLayouts/_rels/slideLayout1.xml.rels");
    zip.start_file(
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        FileOptions::default(),
    )?;
    zip.write_all(&build_slide_layout_rels_xml()?)?;

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
        zip.write_all(&build_slide_rels_xml(&image_name)?)?;

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
        zip.write_all(&build_slide_xml(slide_num, cx, cy)?)?;
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
