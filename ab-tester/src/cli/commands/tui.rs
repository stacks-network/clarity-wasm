use std::{
    io::{stdout, Stdout}, 
    time::Duration
};

use anyhow::{Result, Context};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

use crate::{ok, cli::TuiArgs};

struct App {

}

impl App {
    pub fn new() -> Self {
        Self{}
    }
}

pub fn exec(config: &crate::config::Config, args: TuiArgs) -> Result<()> {
    // Prepare the terminal for the Terminal UI
    let mut terminal = setup_terminal()
        .context("terminal setup failed")?;

    // Create app and run it
    let app = App::new();

    // We store the result here instead of using a '?' because we want to make
    // sure that we restore the terminal before returning from this function.
    let res = run(&mut terminal, app)
        .context("app loop failed");

    // Restore terminal
    restore_terminal(terminal).context("restore terminal failed")?;

    // Now we can handle errors.
    res?;

    ok!()
}

/// Prepares the terminal for a TUI application.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).context("unable to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)
        .context("failed to create terminal")?;

    Ok(terminal)
}

/// Restores the terminal to the state before the TUI application was launched.
fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("unable to return to main screen")?;
    terminal.show_cursor().context("failed to show cursor")?;

    ok!()
}

fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App
) -> Result<()> {
    loop {
        terminal.draw(|f| render_ui(f, &app))?;

        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return ok!();
                }
            }
        }
    }
}

fn render_ui<B: Backend>(frame: &mut Frame<B>, app: &App) {
    let greeting = Paragraph::new("Hello World! (press 'q' to quit)");
    frame.render_widget(greeting, frame.size());
}