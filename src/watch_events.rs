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
    FileChanged { path: PathBuf, timestamp: Instant },
    /// Regeneration started
    RegenerationStarted { timestamp: Instant },
    /// Regeneration completed
    RegenerationCompleted {
        slide_count: usize,
        slide_titles: Vec<String>,
        duration_ms: u64,
        changed_slide: Option<usize>,
    },
    /// Regeneration failed
    RegenerationFailed { error: String },
    /// Server status update
    ServerStatus {
        http_port: u16,
        ws_port: Option<u16>,
        connected_clients: usize,
    },
    /// A WebSocket client connected or disconnected
    ClientCountChanged { count: usize },
}

/// WebSocket message sent to browser for reload with optional slide focus
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"action":"reload"}"#.to_string())
    }
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

    #[test]
    fn test_watch_event_file_changed() {
        let event = WatchEvent::FileChanged {
            path: PathBuf::from("slides.md"),
            timestamp: Instant::now(),
        };
        match event {
            WatchEvent::FileChanged { path, .. } => {
                assert_eq!(path, PathBuf::from("slides.md"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_watch_event_regeneration_failed() {
        let event = WatchEvent::RegenerationFailed {
            error: "browser crashed".into(),
        };
        match event {
            WatchEvent::RegenerationFailed { error } => {
                assert_eq!(error, "browser crashed");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_watch_event_server_status() {
        let event = WatchEvent::ServerStatus {
            http_port: 8080,
            ws_port: Some(8081),
            connected_clients: 2,
        };
        match event {
            WatchEvent::ServerStatus {
                http_port,
                ws_port,
                connected_clients,
            } => {
                assert_eq!(http_port, 8080);
                assert_eq!(ws_port, Some(8081));
                assert_eq!(connected_clients, 2);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_watch_event_client_count_changed() {
        let event = WatchEvent::ClientCountChanged { count: 3 };
        match event {
            WatchEvent::ClientCountChanged { count } => {
                assert_eq!(count, 3);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_reload_message_serialize() {
        let msg = ReloadMessage::reload_with_focus(3);
        let json = msg.to_json();
        assert!(json.contains("\"action\":\"reload\""));
        assert!(json.contains("\"focus_slide\":3"));
    }

    #[test]
    fn test_reload_message_no_focus() {
        let msg = ReloadMessage::reload();
        let json = msg.to_json();
        assert!(json.contains("\"action\":\"reload\""));
        assert!(!json.contains("focus_slide"));
    }

    #[test]
    fn test_reload_message_roundtrip() {
        let msg = ReloadMessage::reload_with_focus(5);
        let json = msg.to_json();
        let deserialized: ReloadMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
