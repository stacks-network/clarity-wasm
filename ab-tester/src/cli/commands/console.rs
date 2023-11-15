use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::Result;

use crate::cli::console::app::{App, AppState};
use crate::cli::console::screens::main::MainLayout;
use crate::cli::console::screens::{BlocksScreen, StartScreen};
use crate::cli::console::theme::{ColorScheme, Theme};
use crate::cli::TuiArgs;

pub async fn exec(config: &crate::config::Config, args: TuiArgs) -> Result<()> {
    // Determine the theme to use.
    let mut theme = Theme::default();
    if let Some(theme_str) = args.theme {
        theme = Theme::new(ColorScheme::from(theme_str.as_str()), true);
    } else if let Some(theme_str) = &config.app.console_theme {
        theme = Theme::new(ColorScheme::from(theme_str.as_str()), true);
    }

    let _blocks = Rc::new(RefCell::new(BlocksScreen::new()));
    let start = Rc::new(RefCell::new(StartScreen::new()));
    let main_layout = Rc::new(RefCell::new(MainLayout::new()));

    let mut app_state = AppState::new();
    let mut app = App::new(
        "Stacks A/B Tester Thingy v0.0.0",
        config,
        &theme,
        &mut app_state,
    )?;

    app.register_component(Rc::clone(&main_layout));
    main_layout.borrow_mut().set_body(start);

    app.run().await?;

    Ok(())
}
