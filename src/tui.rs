// ABOUTME: TUI dashboard for watch mode using boba framework
// ABOUTME: Displays presentation stats, server status, activity log, and slide previews

use std::collections::VecDeque;
use std::path::PathBuf;

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::Frame;
use boba::ratatui::layout::{Constraint, Direction, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use boba::{Command, Model, Subscription, TerminalEvent, terminal_events};

use crate::watch_events::WatchEvent;

const MAX_ACTIVITY_LOG: usize = 50;

/// Flags passed to the Dashboard on init
pub struct DashboardFlags {
    /// Initial events to process on startup (e.g. initial stats)
    pub initial_events: Vec<WatchEvent>,
    /// Minutes per slide for estimated presentation time
    pub minutes_per_slide: f64,
}

/// State for the watch mode TUI dashboard
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

    // Activity log (most recent first)
    activity_log: VecDeque<ActivityEntry>,

    // Regeneration state
    is_regenerating: bool,
    last_regen_duration_ms: Option<u64>,
    regen_count: usize,
    last_changed_slide: Option<usize>,
}

/// A single entry in the activity log
#[derive(Clone, Debug)]
pub struct ActivityEntry {
    pub timestamp: String,
    pub message: String,
    pub is_error: bool,
}

/// Messages handled by the Dashboard
#[derive(Debug)]
pub enum Msg {
    /// A watch event arrived from the background system
    WatchEvent(WatchEvent),
    /// Quit the TUI
    Quit,
    /// No-op (discard unhandled input)
    Noop,
}

impl Dashboard {
    /// Push a new entry to the front of the activity log, trimming if over capacity
    fn add_activity(&mut self, message: String, is_error: bool) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.activity_log.push_front(ActivityEntry {
            timestamp,
            message,
            is_error,
        });
        while self.activity_log.len() > MAX_ACTIVITY_LOG {
            self.activity_log.pop_back();
        }
    }

    /// Format estimated presentation time based on slide count and pace
    fn estimated_time(&self) -> String {
        let minutes = self.slide_count as f64 * self.minutes_per_slide;
        if minutes < 1.0 {
            format!("{:.0}s", minutes * 60.0)
        } else if minutes < 60.0 {
            format!("{:.0}m", minutes)
        } else {
            let hours = (minutes / 60.0).floor();
            let remaining = minutes % 60.0;
            format!("{:.0}h {:.0}m", hours, remaining)
        }
    }

    /// Process a single WatchEvent, updating dashboard state and activity log
    fn process_event(&mut self, event: WatchEvent) {
        match event {
            WatchEvent::InitialStats {
                slide_count,
                slide_titles,
                markdown_path,
                html_output,
            } => {
                self.slide_count = slide_count;
                self.slide_titles = slide_titles;
                self.markdown_path = markdown_path;
                self.html_output = html_output;
                self.add_activity(format!("Loaded {} slides", slide_count), false);
            }
            WatchEvent::FileChanged { path, .. } => {
                let filename = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                self.add_activity(format!("File changed: {}", filename), false);
            }
            WatchEvent::RegenerationStarted { .. } => {
                self.is_regenerating = true;
                self.add_activity("Regenerating...".to_string(), false);
            }
            WatchEvent::RegenerationCompleted {
                slide_count,
                slide_titles,
                duration_ms,
                changed_slide,
            } => {
                self.is_regenerating = false;
                self.slide_count = slide_count;
                self.slide_titles = slide_titles;
                self.last_regen_duration_ms = Some(duration_ms);
                self.regen_count += 1;
                self.last_changed_slide = changed_slide;
                let slide_info = match changed_slide {
                    Some(idx) => format!(" (slide {} changed)", idx + 1),
                    None => String::new(),
                };
                self.add_activity(
                    format!(
                        "Regenerated ({}ms) - {} slides{}",
                        duration_ms, slide_count, slide_info
                    ),
                    false,
                );
            }
            WatchEvent::RegenerationFailed { error } => {
                self.is_regenerating = false;
                self.add_activity(format!("Error: {}", error), true);
            }
            WatchEvent::ServerStatus {
                http_port,
                ws_port,
                connected_clients,
            } => {
                self.http_port = Some(http_port);
                self.ws_port = ws_port;
                self.connected_clients = connected_clients;
            }
            WatchEvent::ClientCountChanged { count } => {
                self.connected_clients = count;
                self.add_activity(format!("Clients: {}", count), false);
            }
        }
    }
}

impl Model for Dashboard {
    type Message = Msg;
    type Flags = DashboardFlags;

    fn init(flags: DashboardFlags) -> (Self, Command<Msg>) {
        let mut dashboard = Dashboard {
            slide_count: 0,
            slide_titles: Vec::new(),
            minutes_per_slide: flags.minutes_per_slide,
            http_port: None,
            ws_port: None,
            connected_clients: 0,
            markdown_path: PathBuf::new(),
            html_output: PathBuf::new(),
            activity_log: VecDeque::new(),
            is_regenerating: false,
            last_regen_duration_ms: None,
            regen_count: 0,
            last_changed_slide: None,
        };

        // Process any initial events provided at startup
        for event in flags.initial_events {
            dashboard.process_event(event);
        }

        (dashboard, Command::none())
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::WatchEvent(event) => {
                self.process_event(event);
                Command::none()
            }
            Msg::Quit => Command::quit(),
            Msg::Noop => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: top info bar, middle slide list, bottom activity log
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Top info panels
                Constraint::Min(5),     // Slide list
                Constraint::Length(12), // Activity log
            ])
            .split(area);

        // Top section: split into stats (left) and server status (right)
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[0]);

        // === Stats Panel (top-left) ===
        let status_indicator = if self.is_regenerating {
            Span::styled(
                " BUILDING ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            )
        } else {
            Span::styled(
                " WATCHING ",
                Style::default().fg(Color::Black).bg(Color::Green),
            )
        };

        let stats_text = vec![
            Line::from(vec![Span::raw("  Status: "), status_indicator]),
            Line::from(vec![
                Span::raw("  Slides: "),
                Span::styled(
                    format!("{}", self.slide_count),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Est. time: "),
                Span::styled(
                    self.estimated_time(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Rebuilds: "),
                Span::styled(
                    format!("{}", self.regen_count),
                    Style::default().fg(Color::White),
                ),
                match self.last_regen_duration_ms {
                    Some(ms) => Span::styled(
                        format!(" (last: {}ms)", ms),
                        Style::default().fg(Color::DarkGray),
                    ),
                    None => Span::raw(""),
                },
            ]),
        ];

        let stats_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" big-rs watch ");

        let stats_paragraph = Paragraph::new(stats_text).block(stats_block);
        frame.render_widget(stats_paragraph, top_chunks[0]);

        // === Server Panel (top-right) ===
        let http_url = match self.http_port {
            Some(port) => format!("http://localhost:{}", port),
            None => "not running".to_string(),
        };
        let ws_url = match self.ws_port {
            Some(port) => format!("ws://localhost:{}", port),
            None => "not running".to_string(),
        };

        let server_text = vec![
            Line::from(vec![
                Span::raw("  HTTP: "),
                Span::styled(&http_url, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("  WS:   "),
                Span::styled(&ws_url, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("  Clients: "),
                Span::styled(
                    format!("{}", self.connected_clients),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" quit"),
            ]),
        ];

        let server_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Server ");

        let server_paragraph = Paragraph::new(server_text).block(server_block);
        frame.render_widget(server_paragraph, top_chunks[1]);

        // === Slide List (middle) ===
        let slide_items: Vec<ListItem> = self
            .slide_titles
            .iter()
            .enumerate()
            .map(|(i, title)| {
                let is_changed = self.last_changed_slide == Some(i);
                let prefix = if is_changed { " >> " } else { "    " };
                let number = format!("{}. ", i + 1);

                let style = if is_changed {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(number, Style::default().fg(Color::DarkGray)),
                    Span::styled(title.clone(), style),
                ]))
            })
            .collect();

        let slides_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" Slides ({}) ", self.slide_count));

        let slides_list = List::new(slide_items).block(slides_block);
        frame.render_widget(slides_list, main_chunks[1]);

        // === Activity Log (bottom) ===
        let log_items: Vec<ListItem> = self
            .activity_log
            .iter()
            .map(|entry| {
                let style = if entry.is_error {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(&entry.message, style),
                ]))
            })
            .collect();

        let log_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Activity ");

        let log_list = List::new(log_items).block(log_block);
        frame.render_widget(log_list, main_chunks[2]);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![terminal_events(|ev| match ev {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Msg::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Msg::Quit)
                }
                _ => Some(Msg::Noop),
            },
            _ => None,
        })]
    }
}
