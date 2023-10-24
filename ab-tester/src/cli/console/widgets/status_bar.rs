use ratatui::widgets::{Widget, Paragraph};

use crate::cli::console::theme::Theme;

pub struct StatusBar<'theme> {
    theme: &'theme Theme
}

impl<'theme> StatusBar<'theme> {
    pub fn new(theme: &'theme Theme) -> Self {
        Self { theme }
    }
}

impl Widget for &StatusBar<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        Paragraph::new("[env: baseline]")
            .style(self.theme.header).render(area, buf)
    }
}