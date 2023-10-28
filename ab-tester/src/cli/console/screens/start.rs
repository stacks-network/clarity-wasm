use crossterm::event::KeyEvent;
use ratatui::{widgets::{Paragraph, Block, Borders, BorderType}, prelude::{Rect, Layout, Direction, Constraint}};
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

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Min(10)
            ].as_ref())
            .split(area);

        let left_pane = rects[0];
        let right_pane = rects[1];

        self.draw_environments_pane(f, rects[0], theme)?;


        Ok(())
    }
}

impl StartScreen {
    fn draw_environments_pane(&mut self, f: &mut Frame<'_>, area: Rect, _theme: &Theme) -> Result<()> {
        let widget = Block::new()
            .title("Environments")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
        ;

        f.render_widget(widget, area);

        Ok(())
    }

    fn draw_config_pane(&mut self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Result<()> {
        let widget = Block::new()
        ;
        Ok(())
    }
}