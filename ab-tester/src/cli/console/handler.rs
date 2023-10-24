use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::App;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> Result<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        // Counter handlers
        KeyCode::Right => {
            //app.increment_counter();
        }
        KeyCode::Left => {
            //app.decrement_counter();
        }
        KeyCode::Char('1') => {

        }
        KeyCode::Char('2') => {

        }
        KeyCode::Char('3') => {

        }
        KeyCode::Char('4') => {

        },
        KeyCode::Char('e') => {
            let mut screen = app.current_screen_inst.borrow_mut();
            screen.handle_key_event(key_event);
        }
        _ => {}
    }
    Ok(())
}
