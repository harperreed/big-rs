// ABOUTME: Integration tests for TUI dashboard features
// ABOUTME: Tests diff module, reload messages, and watch event communication

use big_slides::diff;
use big_slides::watch_events::{ReloadMessage, WatchEvent};
use std::path::PathBuf;
use std::time::Instant;

/// Test diff with realistic multi-slide markdown
#[test]
fn test_diff_realistic_presentation() {
    let markdown = r#"% My Presentation
% Author Name
% 2024-01-01

# Introduction

Welcome to the presentation.

This has multiple lines.

---

# Architecture Overview

Here's our system design:

- Component A
- Component B
- Component C

---

# Implementation Details

The code uses Rust with:

```rust
fn main() {
    println!("hello");
}
```

---

# Conclusion

Thank you for listening.

Questions?
"#;

    let slide_count = diff::count_slides(markdown);
    assert!(
        slide_count >= 4,
        "Should have at least 4 slides, got {}",
        slide_count
    );

    let titles = diff::extract_slide_titles(markdown);
    assert!(
        titles.len() >= 4,
        "Should have at least 4 titles, got {}",
        titles.len()
    );

    // Titles should be meaningful (not empty)
    for (i, title) in titles.iter().enumerate() {
        assert!(!title.is_empty(), "Slide {} title should not be empty", i);
    }
}

/// Test diff detects changes in middle of presentation
#[test]
fn test_diff_change_in_middle_slide() {
    let old = "# Slide 1\n\nIntro\n\n---\n\n# Slide 2\n\nOld content here\n\n---\n\n# Slide 3\n\nConclusion";
    let new = "# Slide 1\n\nIntro\n\n---\n\n# Slide 2\n\nUpdated content here\n\n---\n\n# Slide 3\n\nConclusion";

    let changed = diff::find_changed_slide(old, new);
    assert_eq!(
        changed,
        Some(1),
        "Should detect change in slide 2 (index 1)"
    );
}

/// Test diff handles adding new slide at end
#[test]
fn test_diff_add_slide_at_end() {
    let old = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nContent";
    let new =
        "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nContent\n\n---\n\n# Slide 3\n\nNew slide";

    let old_count = diff::count_slides(old);
    let new_count = diff::count_slides(new);
    assert_eq!(old_count, 2);
    assert_eq!(new_count, 3);

    let changed = diff::find_changed_slide(old, new);
    assert_eq!(changed, Some(2), "Should detect new slide at index 2");
}

/// Test diff handles removing a slide
#[test]
fn test_diff_remove_slide() {
    let old = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nContent\n\n---\n\n# Slide 3\n\nContent";
    let new = "# Slide 1\n\nContent\n\n---\n\n# Slide 3\n\nContent";

    let changed = diff::find_changed_slide(old, new);
    assert!(changed.is_some(), "Should detect that slides changed");
}

/// Test ReloadMessage JSON roundtrip
#[test]
fn test_reload_message_json_roundtrip() {
    // With focus
    let msg = ReloadMessage::reload_with_focus(7);
    let json = msg.to_json();
    let parsed: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.action, "reload");
    assert_eq!(parsed.focus_slide, Some(7));

    // Without focus
    let msg = ReloadMessage::reload();
    let json = msg.to_json();
    let parsed: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.action, "reload");
    assert_eq!(parsed.focus_slide, None);

    // Verify no focus_slide key when None
    assert!(
        !json.contains("focus_slide"),
        "JSON should omit focus_slide when None"
    );
}

/// Test ReloadMessage JSON format is browser-compatible
#[test]
fn test_reload_message_browser_compatible_format() {
    let msg = ReloadMessage::reload_with_focus(3);
    let json = msg.to_json();

    // Parse as generic JSON value to verify structure
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value.is_object());
    assert_eq!(value["action"], "reload");
    assert_eq!(value["focus_slide"], 3);
}

/// Test WatchEvent variants can be constructed and matched
#[test]
fn test_watch_event_variants() {
    let events = vec![
        WatchEvent::InitialStats {
            slide_count: 10,
            slide_titles: vec!["A".into(), "B".into()],
            markdown_path: PathBuf::from("test.md"),
            html_output: PathBuf::from("test.html"),
        },
        WatchEvent::FileChanged {
            path: PathBuf::from("test.md"),
            timestamp: Instant::now(),
        },
        WatchEvent::RegenerationStarted {
            timestamp: Instant::now(),
        },
        WatchEvent::RegenerationCompleted {
            slide_count: 10,
            slide_titles: vec!["A".into(), "B".into()],
            duration_ms: 42,
            changed_slide: Some(3),
        },
        WatchEvent::RegenerationFailed {
            error: "test error".into(),
        },
        WatchEvent::ServerStatus {
            http_port: 8080,
            ws_port: Some(8081),
            connected_clients: 1,
        },
        WatchEvent::ClientCountChanged { count: 2 },
    ];

    assert_eq!(events.len(), 7, "Should have all 7 event variants");
}

/// Test WatchEvent channel communication via tokio mpsc
#[test]
fn test_watch_event_channel_communication() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WatchEvent>();

    // Send events
    tx.send(WatchEvent::InitialStats {
        slide_count: 5,
        slide_titles: vec!["Title".into()],
        markdown_path: PathBuf::from("slides.md"),
        html_output: PathBuf::from("slides.html"),
    })
    .unwrap();

    tx.send(WatchEvent::RegenerationCompleted {
        slide_count: 5,
        slide_titles: vec!["Title".into()],
        duration_ms: 100,
        changed_slide: Some(0),
    })
    .unwrap();

    // Receive events
    let event1 = rx.try_recv().unwrap();
    match event1 {
        WatchEvent::InitialStats { slide_count, .. } => {
            assert_eq!(slide_count, 5);
        }
        _ => panic!("Expected InitialStats"),
    }

    let event2 = rx.try_recv().unwrap();
    match event2 {
        WatchEvent::RegenerationCompleted {
            duration_ms,
            changed_slide,
            ..
        } => {
            assert_eq!(duration_ms, 100);
            assert_eq!(changed_slide, Some(0));
        }
        _ => panic!("Expected RegenerationCompleted"),
    }
}

/// Test slide titles extraction with mixed header formats
#[test]
fn test_extract_titles_mixed_formats() {
    let md = "# Standard Header\n\nContent\n\n---\n\n#NoSpaceHeader\n\nContent\n\n---\n\nPlain text slide";
    let titles = diff::extract_slide_titles(md);
    assert_eq!(titles.len(), 3);
    assert_eq!(titles[0], "Standard Header");
    assert_eq!(titles[1], "NoSpaceHeader");
    assert_eq!(titles[2], "Plain text slide");
}

/// Test slide line mapping is consistent with splitting
#[test]
fn test_slide_line_mapping_consistency() {
    let md =
        "# Slide 1\n\nContent 1\n\n---\n\n# Slide 2\n\nContent 2\n\n---\n\n# Slide 3\n\nContent 3";
    let slides = diff::split_into_slides(md);
    let line_ranges = diff::map_slides_to_lines(md);

    assert_eq!(
        slides.len(),
        line_ranges.len(),
        "Number of slides should match number of line ranges"
    );
}
