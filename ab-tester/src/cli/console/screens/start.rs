use crossterm::event::KeyEvent;
use ratatui::{widgets::Paragraph, prelude::Rect};
use color_eyre::eyre::Result;

use crate::cli::console::{theme::Theme, components::Component, tui::Frame, action::Action};

pub struct StartScreen {
}

impl StartScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for StartScreen {
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        eprintln!("key: {key:?}");
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _theme: &Theme) -> Result<()> {
        let tmp = Paragraph::new("hello");
        f.render_widget(tmp, area);
        Ok(())
    }
}