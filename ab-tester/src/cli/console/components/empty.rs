use super::Component;

pub struct Empty {}

impl Component for Empty {
    fn draw(&mut self, _f: &mut crate::cli::console::tui::Frame<'_>, _area: ratatui::prelude::Rect, _theme: &crate::cli::console::theme::Theme) -> color_eyre::eyre::Result<()> {
        Ok(())
    }
}