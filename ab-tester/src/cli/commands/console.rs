use std::io;

use anyhow::Result;
use ratatui::prelude::*;

use crate::cli::{
    console::{
        app::{App, AppState},
        event::{Event, EventHandler},
        handler::handle_key_events,
        tui::Tui, theme::{Theme, ColorScheme},
    },
    TuiArgs,
};

pub fn exec(config: &crate::config::Config, args: TuiArgs) -> Result<()> {
    // Determine the theme to use.
    let mut theme = Theme::default();
    if let Some(theme_str) = args.theme {
        theme = Theme::new(ColorScheme::from(theme_str.as_str()), true);
    } else if let Some(theme_str) = &config.app.console_theme {
        theme = Theme::new(ColorScheme::from(theme_str.as_str()), true);
    }

    // Create the application.
    let app_state = AppState::new();
    let mut app = App::new("Stacks A/B Tester Thingy v0.0.0",
        &theme, 
        app_state);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());

    // Create the terminal instance using the selected backend.
    let mut terminal = Terminal::new(backend)?;
    
    // Set the theme.
    let frame = terminal.get_frame().size();
    terminal.current_buffer_mut().set_style(frame, app.styles.background);

    // Setup the event handler for this application's events.
    let events = EventHandler::new(250);

    // Go!
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.state.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
