use ratatui::prelude::*;

use super::{
    app::{App, AppScreen}, 
    MIN_WIDTH, MIN_HEIGHT
};

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    let size = frame.size();

    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        frame.render_widget(&app.widgets.area_warning, size);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ].as_ref())
        .split(size);

    // Render the header.
    frame.render_widget(&app.widgets.header, layout[0]);

    // Render the current screen.
    match app.current_screen {
        AppScreen::Blocks | AppScreen::Default => {
            frame.render_widget(&mut app.screens.blocks, layout[1]);
        }
    }

    // Render the menu.
    frame.render_widget(&app.widgets.menu, layout[2]);
    // Render the status bar.
    frame.render_widget(&app.widgets.status_bar, layout[3]);

    /*let popup = Block::default()
        .title(title_span("Manage Environments", app.styles.popup_title, Style::default()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(app.styles.popup_content)
        ;
    let popup_area = centered_rect(80, 50, size);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);*/
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}