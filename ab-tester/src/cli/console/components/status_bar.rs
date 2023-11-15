use ratatui::widgets::Paragraph;

use super::Component;
use crate::cli::console::theme::Theme;

pub struct StatusBar {}

impl StatusBar {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for StatusBar {
    fn draw(
        &mut self,
        f: &mut crate::cli::console::tui::Frame<'_>,
        area: ratatui::prelude::Rect,
        theme: &Theme,
    ) -> color_eyre::eyre::Result<()> {
        let widget = Paragraph::new("[env: baseline]").style(theme.header);

        f.render_widget(widget, area);

        Ok(())
    }
}
