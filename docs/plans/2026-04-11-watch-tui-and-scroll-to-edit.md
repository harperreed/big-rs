# Watch TUI Dashboard & Scroll-to-Edit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace watch mode's plain log output with a boba TUI dashboard showing presentation stats, server status, live activity, and slide previews — plus diff-based scroll-to-edit that navigates the browser preview to the most recently changed slide.

**Architecture:** The existing watch mode spawns background threads for HTTP serving and WebSocket. We add a boba TUI as the foreground, fed by an mpsc channel from the watch/regeneration threads. The TUI is read-only monitoring — no interactive controls beyond quit. Scroll-to-edit diffs old vs new markdown on each save, maps the first changed line to a slide index, and sends that index over WebSocket as a JSON message instead of bare "reload".

**Tech Stack:** boba (TUI framework, Elm Architecture on ratatui/crossterm/tokio), existing notify + tiny_http + tungstenite stack unchanged.

---

## Task 1: Add boba dependency and create TUI module skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/tui.rs`
- Modify: `src/lib.rs`

**Step 1: Add boba to Cargo.toml**

Add under `[dependencies]`:
```toml
boba = { git = "https://github.com/2389-research/boba.git" }
```

Also add `serde` and `serde_json` for the WebSocket JSON messages:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Step 2: Create src/tui.rs with ABOUTME header and empty Model**

```rust
// ABOUTME: TUI dashboard for watch mode using boba framework
// ABOUTME: Displays presentation stats, server status, activity log, and slide previews

use boba::ratatui::Frame;
use boba::{Command, Model, Subscription};

/// State for the watch mode TUI dashboard
pub struct Dashboard {
}

#[derive(Debug)]
pub enum Msg {
    Quit,
}

impl Model for Dashboard {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        (Dashboard {}, Command::none())
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, _frame: &mut Frame) {}

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![]
    }
}
```

**Step 3: Register module in lib.rs**

Add `pub mod tui;` to `src/lib.rs`.

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors (possibly warnings about unused code, that's fine).

**Step 5: Commit**

```bash
git add Cargo.toml src/tui.rs src/lib.rs
git commit -m "feat: add boba dependency and TUI module skeleton"
```

---

## Task 2: Define shared state types and mpsc channel for watch events

**Files:**
- Create: `src/watch_events.rs`
- Modify: `src/lib.rs`

**Step 1: Write the failing test**

Create `src/watch_events.rs` with tests:

```rust
// ABOUTME: Event types shared between the watch system and TUI dashboard
// ABOUTME: Provides channel-based communication for watch status updates

use std::path::PathBuf;
use std::time::Instant;

/// Events sent from watch/regeneration threads to the TUI
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Initial stats after first generation
    InitialStats {
        slide_count: usize,
        slide_titles: Vec<String>,
        markdown_path: PathBuf,
        html_output: PathBuf,
    },
    /// A file change was detected
    FileChanged {
        path: PathBuf,
        timestamp: Instant,
    },
    /// Regeneration started
    RegenerationStarted {
        timestamp: Instant,
    },
    /// Regeneration completed
    RegenerationCompleted {
        slide_count: usize,
        slide_titles: Vec<String>,
        duration_ms: u64,
        changed_slide: Option<usize>,
    },
    /// Regeneration failed
    RegenerationFailed {
        error: String,
    },
    /// Server status update
    ServerStatus {
        http_port: u16,
        ws_port: Option<u16>,
        connected_clients: usize,
    },
    /// A WebSocket client connected or disconnected
    ClientCountChanged {
        count: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_event_initial_stats() {
        let event = WatchEvent::InitialStats {
            slide_count: 5,
            slide_titles: vec!["Intro".into(), "Main".into()],
            markdown_path: PathBuf::from("slides.md"),
            html_output: PathBuf::from("slides.html"),
        };
        match event {
            WatchEvent::InitialStats { slide_count, .. } => {
                assert_eq!(slide_count, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_watch_event_regeneration_completed() {
        let event = WatchEvent::RegenerationCompleted {
            slide_count: 10,
            slide_titles: vec!["A".into()],
            duration_ms: 150,
            changed_slide: Some(3),
        };
        match event {
            WatchEvent::RegenerationCompleted {
                changed_slide,
                duration_ms,
                ..
            } => {
                assert_eq!(changed_slide, Some(3));
                assert_eq!(duration_ms, 150);
            }
            _ => panic!("wrong variant"),
        }
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test watch_event`
Expected: 2 tests pass.

**Step 3: Register module in lib.rs**

Add `pub mod watch_events;` and `pub use watch_events::WatchEvent;` to `src/lib.rs`.

**Step 4: Verify full build**

Run: `cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add src/watch_events.rs src/lib.rs
git commit -m "feat: add WatchEvent types for watch-to-TUI communication"
```

---

## Task 3: Implement slide diffing for scroll-to-edit

**Files:**
- Create: `src/diff.rs`
- Modify: `src/lib.rs`

This module compares old and new markdown content, finds the first changed line, and maps it to a slide index.

**Step 1: Write the failing tests**

```rust
// ABOUTME: Diff-based change detection for scroll-to-edit functionality
// ABOUTME: Compares markdown versions to identify which slide was edited

/// Split markdown content into slides by "---" separator and return
/// the line ranges for each slide (start_line, end_line) where lines
/// are 0-indexed from the original content.
pub fn map_slides_to_lines(content: &str) -> Vec<(usize, usize)> {
    todo!()
}

/// Given old and new markdown content, find the 0-based index of the
/// first slide that changed. Returns None if nothing changed.
pub fn find_changed_slide(old_content: &str, new_content: &str) -> Option<usize> {
    todo!()
}

/// Extract slide titles from markdown content (first # header in each slide section)
pub fn extract_slide_titles(content: &str) -> Vec<String> {
    todo!()
}

/// Count the number of slides in markdown content
pub fn count_slides(content: &str) -> usize {
    todo!()
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
    fn test_count_slides_multiple() {
        let md = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nMore";
        assert_eq!(count_slides(md), 2);
    }

    #[test]
    fn test_count_slides_with_headers_as_separators() {
        // Headers also create slide breaks in big-rs
        let md = "# Slide 1\n\nContent\n\n# Slide 2\n\nMore";
        assert_eq!(count_slides(md), 2);
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
        let new = "# Slide 1\n\nNew content\n\n---\n\n# Slide 2\n\nContent";
        assert_eq!(find_changed_slide(old, new), Some(0));
    }

    #[test]
    fn test_find_changed_slide_second_slide() {
        let old = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nOld";
        let new = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new), Some(1));
    }

    #[test]
    fn test_find_changed_slide_added_slide() {
        let old = "# Slide 1\n\nContent";
        let new = "# Slide 1\n\nContent\n\n---\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new), Some(1));
    }

    #[test]
    fn test_find_changed_slide_header_separator() {
        let old = "# Slide 1\n\nContent\n\n# Slide 2\n\nOld";
        let new = "# Slide 1\n\nContent\n\n# Slide 2\n\nNew";
        assert_eq!(find_changed_slide(old, new), Some(1));
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
}
```

**Step 2: Run tests, confirm they fail**

Run: `cargo test diff`
Expected: All tests fail with `not yet implemented`.

**Step 3: Implement the diff functions**

The key insight: slides are separated by `---` on its own line OR by `#` headers (which big-rs's `process_content_for_slides` converts to `---` separators). We need to replicate that splitting logic.

```rust
pub fn split_into_slides(content: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut slides: Vec<String> = Vec::new();
    let mut current_slide = Vec::new();
    let mut is_first = true;

    for line in &lines {
        let trimmed = line.trim();

        // Check for --- separator
        if trimmed == "---" {
            if !current_slide.is_empty() {
                slides.push(current_slide.join("\n"));
                current_slide.clear();
            }
            continue;
        }

        // Check for # header (slide break, same logic as html.rs process_content_for_slides)
        let is_header = trimmed.starts_with('#')
            && (trimmed.len() == 1
                || (trimmed.len() > 1
                    && trimmed.chars().nth(1) != Some('#')
                    && (trimmed.chars().nth(1) == Some(' ')
                        || !trimmed.chars().nth(1).unwrap().is_whitespace())));

        if is_header && !is_first {
            // Start a new slide
            if !current_slide.is_empty() {
                slides.push(current_slide.join("\n"));
                current_slide.clear();
            }
        }

        if is_header {
            is_first = false;
        }

        current_slide.push(*line);
    }

    if !current_slide.is_empty() {
        slides.push(current_slide.join("\n"));
    }

    if slides.is_empty() {
        slides.push(content.to_string());
    }

    slides
}

pub fn count_slides(content: &str) -> usize {
    split_into_slides(content).len()
}

pub fn extract_slide_titles(content: &str) -> Vec<String> {
    split_into_slides(content)
        .iter()
        .map(|slide| {
            let first_line = slide.lines().next().unwrap_or("");
            let trimmed = first_line.trim();
            if trimmed.starts_with("# ") {
                trimmed[2..].to_string()
            } else if trimmed.starts_with('#') && !trimmed.starts_with("##") {
                trimmed[1..].trim().to_string()
            } else {
                // Use first non-empty line as title
                trimmed.to_string()
            }
        })
        .collect()
}

pub fn map_slides_to_lines(content: &str) -> Vec<(usize, usize)> {
    let lines: Vec<&str> = content.lines().collect();
    let slides = split_into_slides(content);
    let mut ranges = Vec::new();
    let mut current_line = 0;

    for slide in &slides {
        let slide_line_count = slide.lines().count();
        let start = current_line;
        let end = current_line + slide_line_count - 1;
        ranges.push((start, end));

        // Skip past slide content plus any separator lines
        current_line = end + 1;
        // Skip separator lines (--- or blank lines between slides)
        while current_line < lines.len() {
            let trimmed = lines[current_line].trim();
            if trimmed == "---" || trimmed.is_empty() {
                current_line += 1;
            } else {
                break;
            }
        }
    }

    ranges
}

pub fn find_changed_slide(old_content: &str, new_content: &str) -> Option<usize> {
    if old_content == new_content {
        return None;
    }

    let old_slides = split_into_slides(old_content);
    let new_slides = split_into_slides(new_content);

    let max_len = old_slides.len().max(new_slides.len());

    for i in 0..max_len {
        let old_slide = old_slides.get(i).map(|s| s.as_str());
        let new_slide = new_slides.get(i).map(|s| s.as_str());

        if old_slide != new_slide {
            return Some(i);
        }
    }

    None
}
```

**Step 4: Run tests, confirm they pass**

Run: `cargo test diff`
Expected: All tests pass.

**Step 5: Register module in lib.rs**

Add `pub mod diff;` to `src/lib.rs`.

**Step 6: Commit**

```bash
git add src/diff.rs src/lib.rs
git commit -m "feat: add diff module for scroll-to-edit slide detection"
```

---

## Task 4: Update WebSocket messages to support scroll-to-edit

**Files:**
- Modify: `src/watch.rs` (auto_reload_js function and broadcast calls)

**Step 1: Write the failing test for JSON message format**

Add to `src/watch_events.rs`:

```rust
/// WebSocket message sent to browser for reload with optional slide focus
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReloadMessage {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_slide: Option<usize>,
}

impl ReloadMessage {
    pub fn reload() -> Self {
        Self {
            action: "reload".to_string(),
            focus_slide: None,
        }
    }

    pub fn reload_with_focus(slide: usize) -> Self {
        Self {
            action: "reload".to_string(),
            focus_slide: Some(slide),
        }
    }
}

// Add test:
#[test]
fn test_reload_message_serialize() {
    let msg = ReloadMessage::reload_with_focus(3);
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"action\":\"reload\""));
    assert!(json.contains("\"focus_slide\":3"));
}

#[test]
fn test_reload_message_no_focus() {
    let msg = ReloadMessage::reload();
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"action\":\"reload\""));
    assert!(!json.contains("focus_slide"));
}
```

**Step 2: Run tests**

Run: `cargo test reload_message`
Expected: Tests pass.

**Step 3: Update auto_reload_js to handle JSON messages with slide navigation**

In `src/watch.rs`, update the `auto_reload_js` function's `socket.onmessage` handler to parse JSON and navigate to the changed slide:

```javascript
socket.onmessage = function(event) {
    console.log('Received message:', event.data);
    try {
        var msg = JSON.parse(event.data);
        if (msg.action === 'reload') {
            if (msg.focus_slide !== undefined && msg.focus_slide !== null) {
                // Store the target slide before reload
                sessionStorage.setItem('big_focus_slide', msg.focus_slide);
            }
            window.location.reload();
        }
    } catch(e) {
        // Backwards compatibility: plain "reload" string
        if (event.data === 'reload') {
            window.location.reload();
        }
    }
};
```

Also add a DOMContentLoaded handler to navigate to the stored slide after reload:

```javascript
document.addEventListener('DOMContentLoaded', function() {
    var focusSlide = sessionStorage.getItem('big_focus_slide');
    if (focusSlide !== null) {
        sessionStorage.removeItem('big_focus_slide');
        var slideIndex = parseInt(focusSlide, 10);
        // big.js uses body > div elements as slides
        var slides = document.querySelectorAll('body > div');
        if (slideIndex >= 0 && slideIndex < slides.length) {
            // Navigate to the slide using big.js's go() if available
            if (typeof window.go === 'function') {
                window.go(slideIndex);
            } else {
                // Fallback: show only the target slide
                slides.forEach(function(s, i) {
                    s.style.display = i === slideIndex ? '' : 'none';
                });
            }
        }
    }
});
```

**Step 4: Update broadcast calls in watch_markdown to use JSON and include changed slide**

In `src/watch.rs`, in the `watch_markdown` function's event loop:
- Store previous markdown content in a `String`
- On file change, read new content, call `diff::find_changed_slide`
- Broadcast JSON message with `focus_slide` if a changed slide was detected

**Step 5: Verify build**

Run: `cargo build`
Expected: Compiles.

**Step 6: Commit**

```bash
git add src/watch.rs src/watch_events.rs
git commit -m "feat: add scroll-to-edit via JSON WebSocket messages with slide focus"
```

---

## Task 5: Build the TUI Dashboard model with real state

**Files:**
- Modify: `src/tui.rs`

**Step 1: Define the full Dashboard state and messages**

```rust
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::Frame;
use boba::{terminal_events, Command, Model, Subscription, TerminalEvent};

use crate::watch_events::WatchEvent;

const MAX_ACTIVITY_LOG: usize = 50;
const DEFAULT_MINUTES_PER_SLIDE: f64 = 2.0;

pub struct DashboardFlags {
    pub event_rx: tokio::sync::mpsc::UnboundedReceiver<WatchEvent>,
    pub minutes_per_slide: f64,
}

pub struct Dashboard {
    // Presentation stats
    slide_count: usize,
    slide_titles: Vec<String>,
    minutes_per_slide: f64,

    // Server status
    http_port: Option<u16>,
    ws_port: Option<u16>,
    connected_clients: usize,
    markdown_path: PathBuf,
    html_output: PathBuf,

    // Activity log
    activity_log: VecDeque<ActivityEntry>,

    // Regeneration state
    is_regenerating: bool,
    last_regen_duration_ms: Option<u64>,
    regen_count: usize,

    // Event receiver
    event_rx: tokio::sync::mpsc::UnboundedReceiver<WatchEvent>,
}

#[derive(Clone, Debug)]
pub struct ActivityEntry {
    pub timestamp: String,
    pub message: String,
    pub is_error: bool,
}

#[derive(Debug)]
pub enum Msg {
    WatchEvent(WatchEvent),
    Quit,
    Noop,
}
```

**Step 2: Implement Model trait**

The `init` takes `DashboardFlags` to receive the channel receiver. The `update` handles `WatchEvent` variants to update state. The `subscriptions` uses a stream subscription to poll the mpsc receiver.

**Step 3: Run build**

Run: `cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add src/tui.rs
git commit -m "feat: implement Dashboard model with state and event handling"
```

---

## Task 6: Implement TUI layout and rendering

**Files:**
- Modify: `src/tui.rs`

**Step 1: Implement the view function**

Layout using ratatui:
```
┌─────────────────────────────────────────────┐
│  big-rs watch   |  Server Status            │
│                 |  HTTP: http://localhost:... │
│  Slides: 12     |  WS: ws://localhost:...    │
│  Est. time: 24m |  Clients: 1               │
├─────────────────┴───────────────────────────┤
│  Slides                                      │
│  1. Introduction                             │
│  2. Main Topic          ← changed            │
│  3. Conclusion                               │
├──────────────────────────────────────────────┤
│  Activity Log                                │
│  [12:34:05] File changed: slides.md          │
│  [12:34:05] Regenerated (150ms) - 12 slides  │
│  [12:34:05] Sent reload to 1 client(s)       │
└──────────────────────────────────────────────┘
```

Use `Layout::vertical` for the three main sections:
1. Top row: `Layout::horizontal` split into stats panel (left) and server panel (right)
2. Middle: Slide list with titles
3. Bottom: Scrolling activity log

**Step 2: Build and visually test**

Run: `cargo build`
Expected: Compiles.

**Step 3: Commit**

```bash
git add src/tui.rs
git commit -m "feat: implement TUI dashboard layout with stats, slides, and activity panels"
```

---

## Task 7: Wire TUI into watch mode

**Files:**
- Modify: `src/watch.rs`
- Modify: `src/main.rs`

**Step 1: Add --tui flag to WatchArgs**

In `src/main.rs`, add to `WatchArgs`:
```rust
/// Enable TUI dashboard (requires --serve and --auto-reload)
#[arg(long)]
tui: bool,

/// Estimated minutes per slide for time calculation
#[arg(long, default_value = "2.0")]
minutes_per_slide: f64,
```

**Step 2: Add tui and minutes_per_slide fields to WatchConfig**

In `src/watch.rs`, add to `WatchConfig`:
```rust
pub tui: bool,
pub minutes_per_slide: f64,
```

**Step 3: Create watch_markdown_with_tui function**

This is the key integration point. The function:
1. Creates a `tokio::sync::mpsc::unbounded_channel::<WatchEvent>()`
2. Spawns the existing watch logic (file watcher, server, regeneration) in a background tokio task, sending WatchEvents through the channel
3. Runs `boba::run::<Dashboard>(DashboardFlags { event_rx, minutes_per_slide })` as the foreground

The existing `watch_markdown` function stays intact for non-TUI mode.

**Step 4: Update main.rs watch function to route to TUI or plain mode**

```rust
if args.tui {
    // TUI mode requires serve + auto-reload
    let watch_config = WatchConfig {
        // ... same as before but with tui: true
        serve: true,
        auto_reload: true,
        tui: true,
        minutes_per_slide: args.minutes_per_slide,
    };
    big::tui::run_dashboard(watch_config, config)
} else {
    // Existing plain watch mode
    big::watch_markdown(watch_config, config)
}
```

**Step 5: Build and test manually**

Run: `cargo build`
Then manually test: `cargo run -- watch --input example-preso/slides.md --output /tmp/test.html --tui`

**Step 6: Commit**

```bash
git add src/watch.rs src/main.rs src/tui.rs
git commit -m "feat: wire TUI dashboard into watch mode with --tui flag"
```

---

## Task 8: Integration tests

**Files:**
- Modify: `tests/integration_test.rs` (or create new test file)

**Step 1: Write integration test for diff + slide counting**

Test that the diff module correctly handles real-world markdown content matching what big-rs generates.

**Step 2: Write integration test for ReloadMessage serialization roundtrip**

Test serialize → deserialize preserves all fields.

**Step 3: Write integration test for WatchEvent channel communication**

Test that events flow through an mpsc channel correctly.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 5: Run clippy and fmt**

Run: `cargo fmt && cargo clippy -- -D warnings`
Expected: No errors.

**Step 6: Commit**

```bash
git add tests/
git commit -m "test: add integration tests for TUI events, diff, and reload messages"
```

---

## Task 9: Make --serve and --auto-reload default when --tui is passed

**Files:**
- Modify: `src/main.rs`

When `--tui` is passed, automatically enable `--serve` and `--auto-reload` since the TUI dashboard's server status panel would be useless without them. Log a message if the user didn't explicitly pass these flags.

**Step 1: Update watch function in main.rs**

```rust
fn watch(args: &WatchArgs, config: &big::Config) -> BigResult<()> {
    let serve = args.serve || args.tui;
    let auto_reload = args.auto_reload || args.tui;
    // ... rest of function uses serve/auto_reload instead of args.serve/args.auto_reload
}
```

**Step 2: Build and test**

Run: `cargo build && cargo run -- watch --help`
Expected: Shows --tui flag in help.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: auto-enable serve and auto-reload when TUI is active"
```

---

## Summary of deliverables

1. **boba TUI dashboard** — 4-panel layout showing presentation stats, server status, slide list, activity log
2. **Scroll-to-edit** — diff-based change detection that navigates browser to the edited slide
3. **JSON WebSocket protocol** — backwards-compatible upgrade from plain "reload" to JSON messages
4. **New CLI flags** — `--tui` and `--minutes-per-slide`
5. **Full test coverage** — unit tests for diff module, event types, message serialization, plus integration tests
