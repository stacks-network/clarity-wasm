use std::marker::PhantomData;

use ratatui::{widgets::*, style::Style, text::Line};

use crate::context::BlockCursor;

use super::{theme::Theme, widgets::*, MIN_WIDTH, MIN_HEIGHT, screens::{*, self}, Screen};

/// Application.
pub struct App<'theme, 'a> {
    // App state
    pub state: AppState<'a>,

    // Widgets & Screens
    pub widgets: AppWidgets<'theme, 'a>,
    pub screens: AppScreens<'theme, 'a>,
    pub current_screen: AppScreen,
    pub current_screen_inst: Box<dyn Screen>,
    
    // Styling
    pub styles: AppStyles
}

impl<'theme, 'a> App<'theme, 'a> {
    /// Constructs a new instance of [`App`].
    pub fn new(title: &'a str, theme: &'theme Theme, state: AppState<'a>) -> Self {
        Self {
            // State
            state,

            // Widgets
            widgets: AppWidgets {
                header: Header::new(theme, title),
                menu: Menu::new(theme, vec!["Main", "Blocks", "Transactions", "Contracts", "Load Data"]),
                area_warning: AreaWarning::new(theme, MIN_WIDTH, MIN_HEIGHT),
                status_bar: StatusBar::new(theme),
            },

            screens: AppScreens { 
                blocks: BlocksScreen::new(theme),
            },
            current_screen: AppScreen::Default,
            current_screen_inst: Box::<StartScreen>::default(),

            // Styling
            styles: AppStyles { 
                background: theme.main, 
                popup_title: theme.popup_title, 
                popup_content: theme.popup_fg_bg 
            }
        }
    }

    /*pub fn current_screen_as_widget(&'a self) -> Box<&dyn Widget> {
        let widget: &dyn Widget = Box::new(&*self.current_screen);
    }*/

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.state.running = false;
    }
}

pub struct AppStyles {
    pub background: Style,
    pub popup_title: Style,
    pub popup_content: Style
}

pub struct AppWidgets<'theme, 'a> {
    pub header: Header<'theme, 'a>,
    pub menu: Menu<'theme, 'a>,
    pub area_warning: AreaWarning<'theme>,
    pub status_bar: StatusBar<'theme>,
}

pub struct AppScreens<'theme, 'a> {
    pub blocks: BlocksScreen<'theme, 'a>
}

pub enum AppScreen {
    Default,
    Blocks
}

#[derive(Default)]
pub struct AppState<'data> {
    _lifetime: PhantomData<&'data ()>,
    pub running: bool,

    baseline_block_cursor: Option<&'data BlockCursor>
    
}

impl<'data> AppState<'data> {
    pub fn new() -> Self {
        AppState {
            _lifetime: Default::default(),
            running: true,
            baseline_block_cursor: None
        }
    }
}