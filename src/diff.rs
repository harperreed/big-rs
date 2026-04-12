// ABOUTME: Diff module for detecting slide changes between markdown versions.
// ABOUTME: Provides slide splitting, counting, title extraction, and change detection for scroll-to-edit.

/// Check whether a trimmed line is an h1 header that creates a slide break.
/// Matches the logic in html.rs `process_content_for_slides`:
/// - Must start with `#`
/// - Must NOT be `##` (h2 or deeper)
/// - Accepts both `# Title` (with space) and `#Title` (without space)
/// - Escaped hashes `\#` are handled by pre-processing before this is called
fn is_h1_header(trimmed: &str) -> bool {
    trimmed.starts_with('#')
        && (trimmed.len() == 1
            || (trimmed.len() > 1
                && trimmed.chars().nth(1) != Some('#')
                && (trimmed.chars().nth(1) == Some(' ')
                    || !trimmed.chars().nth(1).unwrap().is_whitespace())))
}

/// Join lines into a slide string, trimming trailing blank lines.
fn join_trimmed(lines: &[&str]) -> String {
    let mut end = lines.len();
    while end > 0 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[..end].join("\n")
}

/// Split markdown content into individual slide sections.
/// Slide breaks occur at `---` separators and `#` headers (not `##`).
/// The first header does not create a break (it starts the first slide).
/// Escaped hashes `\#` are not treated as headers.
pub fn split_into_slides(content: &str) -> Vec<String> {
    // Pre-process escaped hashes so they don't trigger header detection
    let processed = content.replace(r"\#", "!--HASH--!");
    let lines: Vec<&str> = processed.lines().collect();
    let original_lines: Vec<&str> = content.lines().collect();

    let mut slides: Vec<String> = Vec::new();
    let mut current_slide: Vec<&str> = Vec::new();
    let mut seen_header = false;
    // After a `---` separator, the next `#` header should not create
    // another break -- the explicit separator already handled it.
    let mut after_separator = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // `---` separator creates a new slide
        if trimmed == "---" {
            if !current_slide.is_empty() {
                slides.push(join_trimmed(&current_slide));
                current_slide.clear();
            }
            after_separator = true;
            continue;
        }

        let is_header = is_h1_header(trimmed);

        // A header creates a new slide break ONLY when:
        // - we've already seen a header (not the very first one)
        // - we didn't just come from a `---` separator (which already broke)
        // - there's content in the current slide to flush
        if is_header && seen_header && !after_separator && !current_slide.is_empty() {
            slides.push(join_trimmed(&current_slide));
            current_slide.clear();
        }

        if is_header {
            seen_header = true;
            after_separator = false;
        }

        // Skip blank lines between a separator and the start of new slide content
        // (only if nothing has been added to the current slide yet)
        if trimmed.is_empty() && current_slide.is_empty() {
            continue;
        }

        // Use the original line (with real `\#`) rather than the processed one
        current_slide.push(original_lines[i]);
    }

    if !current_slide.is_empty() {
        slides.push(join_trimmed(&current_slide));
    }

    if slides.is_empty() {
        slides.push(content.to_string());
    }

    slides
}

/// Count the number of slides in markdown content.
pub fn count_slides(content: &str) -> usize {
    split_into_slides(content).len()
}

/// Extract the title for each slide. If a slide has a `#` header, use that;
/// otherwise use the first non-empty line of the slide.
pub fn extract_slide_titles(content: &str) -> Vec<String> {
    split_into_slides(content)
        .iter()
        .map(|slide| {
            for line in slide.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Check if this line is an h1 header
                let processed = trimmed.replace(r"\#", "!--HASH--!");
                if is_h1_header(&processed) {
                    // Strip leading `#` and optional space to get the title text
                    let title = trimmed.trim_start_matches('#').trim();
                    return title.to_string();
                }
                // Not a header -- use the first non-empty line as the title
                return trimmed.to_string();
            }
            String::new()
        })
        .collect()
}

/// Map each slide to its (start_line, end_line) range (0-indexed) in the original content.
pub fn map_slides_to_lines(content: &str) -> Vec<(usize, usize)> {
    let processed = content.replace(r"\#", "!--HASH--!");
    let lines: Vec<&str> = processed.lines().collect();
    let total_lines = lines.len();

    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start: usize = 0;
    let mut seen_header = false;
    let mut after_separator = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // `---` separator creates a new slide
        if trimmed == "---" {
            // Close the current slide at the line before the separator
            if i > start {
                ranges.push((start, i - 1));
            }
            start = i + 1;
            after_separator = true;
            continue;
        }

        let is_header = is_h1_header(trimmed);

        // A header creates a new slide break ONLY when:
        // - we've already seen a header (not the very first one)
        // - we didn't just come from a `---` separator
        if is_header && seen_header && !after_separator {
            if i > start {
                ranges.push((start, i - 1));
            }
            start = i;
        }

        if is_header {
            seen_header = true;
            after_separator = false;
        }
    }

    // Close the last slide
    if start < total_lines {
        ranges.push((start, total_lines - 1));
    } else if ranges.is_empty() {
        // Edge case: empty content
        ranges.push((0, 0));
    }

    ranges
}

/// Find the index of the first slide that differs between two markdown versions.
/// Returns `None` if the content is identical, or `Some(index)` for the first changed slide.
/// If slides were added, returns the index of the first new slide.
pub fn find_changed_slide(old_content: &str, new_content: &str) -> Option<usize> {
    let old_slides = split_into_slides(old_content);
    let new_slides = split_into_slides(new_content);

    let max_len = old_slides.len().max(new_slides.len());

    for i in 0..max_len {
        let old_slide = old_slides.get(i);
        let new_slide = new_slides.get(i);

        if old_slide != new_slide {
            return Some(i);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_slides_single() {
        let md = "# Hello\n\nWorld";
        assert_eq!(count_slides(md), 1);
    }

    #[test]
    fn test_count_slides_multiple_hr() {
        let md = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nMore";
        assert_eq!(count_slides(md), 2);
    }

    #[test]
    fn test_count_slides_with_headers_as_separators() {
        let md = "# Slide 1\n\nContent\n\n# Slide 2\n\nMore";
        assert_eq!(count_slides(md), 2);
    }

    #[test]
    fn test_count_slides_no_space_header() {
        let md = "#Slide 1\n\nContent\n\n#Slide 2\n\nMore";
        assert_eq!(count_slides(md), 2);
    }

    #[test]
    fn test_h2_does_not_split() {
        let md = "# Slide 1\n\n## Subtitle\n\nContent";
        assert_eq!(count_slides(md), 1);
    }

    #[test]
    fn test_extract_slide_titles() {
        let md = "# Introduction\n\nHello\n\n---\n\n# Main Point\n\nDetails\n\n---\n\n# Conclusion\n\nBye";
        let titles = extract_slide_titles(md);
        assert_eq!(titles, vec!["Introduction", "Main Point", "Conclusion"]);
    }

    #[test]
    fn test_extract_slide_titles_no_space() {
        let md = "#Introduction\n\nHello\n\n#Main Point\n\nDetails";
        let titles = extract_slide_titles(md);
        assert_eq!(titles, vec!["Introduction", "Main Point"]);
    }

    #[test]
    fn test_extract_slide_titles_no_header() {
        let md = "Just some text\n\n---\n\nMore text";
        let titles = extract_slide_titles(md);
        assert_eq!(titles, vec!["Just some text", "More text"]);
    }

    #[test]
    fn test_find_changed_slide_no_change() {
        let md = "# Slide 1\n\nContent";
        assert_eq!(find_changed_slide(md, md), None);
    }

    #[test]
    fn test_find_changed_slide_first_slide() {
        let old = "# Slide 1\n\nOld content\n\n---\n\n# Slide 2\n\nContent";
        let new_content = "# Slide 1\n\nNew content\n\n---\n\n# Slide 2\n\nContent";
        assert_eq!(find_changed_slide(old, new_content), Some(0));
    }

    #[test]
    fn test_find_changed_slide_second_slide() {
        let old = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nOld";
        let new_content = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new_content), Some(1));
    }

    #[test]
    fn test_find_changed_slide_added_slide() {
        let old = "# Slide 1\n\nContent";
        let new_content = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new_content), Some(1));
    }

    #[test]
    fn test_find_changed_slide_header_separator() {
        let old = "# Slide 1\n\nContent\n\n# Slide 2\n\nOld";
        let new_content = "# Slide 1\n\nContent\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new_content), Some(1));
    }

    #[test]
    fn test_map_slides_to_lines_single() {
        let md = "# Hello\n\nWorld";
        let ranges = map_slides_to_lines(md);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], (0, 2));
    }

    #[test]
    fn test_map_slides_to_lines_multiple() {
        let md = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nMore";
        let ranges = map_slides_to_lines(md);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_split_empty_content() {
        let md = "";
        let slides = split_into_slides(md);
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_split_preserves_content() {
        let md = "# Slide 1\n\nSome content here\n\n---\n\n# Slide 2\n\nOther content";
        let slides = split_into_slides(md);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].contains("Some content here"));
        assert!(slides[1].contains("Other content"));
    }
}
