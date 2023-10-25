use ratatui::{prelude::*, widgets::*, text::Line};

use crate::cli::console::theme::Theme;

use super::Component;

pub struct Menu<'a> {
    pub titles: Vec<&'a str>,
    selected_tab: usize
}

impl<'a> Menu<'a> {
    pub fn new(titles: Vec<&'a str>) -> Self {
        Menu {
            titles,
            selected_tab: 0
        }
    }

    pub fn select(&mut self, selected_tab: usize) {
        self.selected_tab = selected_tab
    }
}

impl<'a> Component for Menu<'a> {
    fn draw(&mut self, f: &mut crate::cli::console::tui::Frame<'_>, area: Rect, theme: &Theme) -> color_eyre::eyre::Result<()> {
        let mut titles: Vec<Line> = Vec::new();
        for (i, title) in self.titles.iter().enumerate() {

            titles.push(Line::from(vec![
                Span::styled(format!("[{}]", i + 1), theme.warning_text_1),
                Span::styled(format!(" {title}"), theme.menu)
            ]));
        }

        let tabs = Tabs::new(titles)
            .select(self.selected_tab)
            .divider(Span::styled(symbols::line::VERTICAL, theme.menu_divider))
            .highlight_style(theme.menu_highlight);

        f.render_widget(tabs, area);

        Ok(())
    }
}