use ratatui::{prelude::*, widgets::*, text::Line};

use crate::cli::console::theme::Theme;

pub struct Menu<'theme, 'a> {
    pub titles: Vec<&'a str>,
    selected_tab: usize,
    theme: &'theme Theme
}

impl<'theme, 'a> Menu<'theme, 'a> {
    pub fn new(theme: &'theme Theme, titles: Vec<&'a str>) -> Self {
        Menu {
            titles,
            selected_tab: 0,
            theme
        }
    }

    pub fn select(&mut self, selected_tab: usize) {
        self.selected_tab = selected_tab
    }
}

impl<'theme, 'a> Widget for &Menu<'theme, 'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut titles: Vec<Line> = Vec::new();
        for (i, title) in self.titles.iter().enumerate() {

            titles.push(Line::from(vec![
                Span::styled(format!("[{}]", i + 1), self.theme.warning_text_1),
                Span::styled(format!(" {title}"), self.theme.menu)
            ]));
        }

        let tabs = Tabs::new(titles)
            .select(self.selected_tab)
            .divider(Span::styled(symbols::line::VERTICAL, self.theme.menu_divider))
            .highlight_style(self.theme.menu_highlight);

        tabs.render(area, buf);
    }
}