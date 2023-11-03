use ratatui::widgets::Paragraph;

use crate::cli::console::theme::Theme;

use super::Component;

pub struct Header {}

impl Header {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Header {
    fn draw(
        &mut self,
        f: &mut crate::cli::console::tui::Frame<'_>,
        area: ratatui::prelude::Rect,
        theme: &Theme,
    ) -> color_eyre::eyre::Result<()> {
        let widget = Paragraph::new("Stacks A/B Tester Thingy")
            .alignment(ratatui::prelude::Alignment::Center)
            .style(theme.header);

        f.render_widget(widget, area);

        Ok(())
    }
}
