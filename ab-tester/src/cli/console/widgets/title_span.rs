use ratatui::{text::{Line, Span}, style::Style, symbols::line::{TOP_RIGHT, TOP_LEFT}};

pub fn title_span(title: &str, title_style: Style, border_style: Style) -> Line {
    Line::from(
        vec![
            Span::styled(TOP_RIGHT.to_string(), border_style),
            Span::styled(format!(" {} ", title), title_style),
            Span::styled(TOP_LEFT.to_string(), border_style)
        ]
    )
}