// ABOUTME: TUI dashboard for watch mode using boba framework
// ABOUTME: Displays presentation stats, server status, activity log, and slide previews

use boba::ratatui::Frame;
use boba::{Command, Model, Subscription};

/// State for the watch mode TUI dashboard
pub struct Dashboard {}

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
