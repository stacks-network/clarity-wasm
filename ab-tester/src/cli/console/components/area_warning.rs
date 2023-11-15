use ratatui::prelude::Alignment;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use super::title_span;
use crate::cli::console::theme::Theme;

pub struct AreaWarning<'theme> {
    title: String,
    theme: &'theme Theme,
    min_width: u16,
    min_height: u16,
}

impl<'theme> AreaWarning<'theme> {
    pub fn new(theme: &'theme Theme, min_width: u16, min_height: u16) -> Self {
        AreaWarning {
            title: "warning".to_string(),
            theme,
            min_width,
            min_height,
        }
    }
}

impl Widget for &AreaWarning<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let lines = vec![
            Line::from(Span::styled(
                format!(
                    "Current width: {}, current height: {}",
                    area.width, area.height
                ),
                self.theme.warning_text_1,
            )),
            Line::from(Span::styled(
                format!(
                    "Minimum requirement: width: {}, height: {}",
                    self.min_width, self.min_height
                ),
                self.theme.warning_text_2,
            )),
        ];

        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.warning_border)
                    .title(title_span(
                        &self.title,
                        self.theme.warning_title,
                        self.theme.warning_border,
                    )),
            )
            .render(area, buf);
    }
}
