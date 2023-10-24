use ratatui::{widgets::{Widget, Paragraph}, text::{Line, Span}, style::Style};

use crate::cli::console::theme::Theme;

pub struct Header<'theme, 'a> {
    pub theme: &'theme Theme,
    title: &'a str
}

impl<'theme, 'a> Header<'theme, 'a> {
    pub fn new(theme: &'theme Theme, title: &'a str) -> Self {
        Self { theme, title }
    }
}

impl<'theme, 'a> Widget for &Header<'theme, 'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {

        Paragraph::new(self.title)
            .alignment(ratatui::prelude::Alignment::Center)
            .style(self.theme.header).render(area, buf)
    }
}