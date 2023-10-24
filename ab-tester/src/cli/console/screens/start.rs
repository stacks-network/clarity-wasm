use ratatui::widgets::{Widget, Paragraph};

use crate::cli::console::Screen;

#[derive(Default)]
pub struct StartScreen {
}

impl StartScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl Screen for StartScreen {
    fn handle_key_event(&mut self, event: crossterm::event::KeyEvent) {
    }
}

impl Widget for StartScreen {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        Paragraph::new("hello").render(area, buf)
    }
}