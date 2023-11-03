use ratatui::{
    prelude::Alignment,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Widget},
};

use crate::cli::console::theme::Theme;

use super::{centered_rect, title_span};

pub struct Popup<'theme, 'a> {
    pub title: &'a str,
    pub theme: &'theme Theme,
    pub content: Option<Box<dyn Widget>>,
}

impl<'theme, 'a> Popup<'theme, 'a> {
    pub fn new(theme: &'theme Theme, title: &'a str, content: impl Widget + 'static) -> Self {
        Self {
            theme,
            title,
            content: Some(Box::new(content)),
        }
    }

    pub fn set_content(&mut self, content: impl Widget + 'static) {
        self.content = Some(Box::new(content))
    }
}

impl<'theme, 'a> Widget for &Popup<'theme, 'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let popup = Block::default()
            .title(title_span(
                "Manage Environments",
                self.theme.popup_title,
                Style::default(),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(self.theme.popup_fg_bg);
        let popup_area = centered_rect(80, 50, area);
        Clear.render(popup_area, buf);
        popup.render(popup_area, buf);
    }
}
