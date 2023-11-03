use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::Rect, style::Style};
use tokio::sync::mpsc;

use super::{
    action::Action,
    components::Component,
    screens::Screen,
    theme::Theme,
    tui::{Event, Tui},
};
use crate::{config::Config, context::BlockCursor};

/// Application.
pub struct App<'a> {
    pub title: &'a str,
    pub config: &'a Config,
    // App state
    pub state: &'a mut AppState<'a>,

    pub tick_rate: f64,
    pub frame_rate: f64,
    pub components: Vec<Rc<RefCell<dyn Component>>>,
    pub current_screen: Option<Rc<RefCell<dyn Component>>>,
    pub should_quit: bool,
    pub should_suspend: bool,
    pub screen: Screen,
    pub last_tick_key_events: Vec<KeyEvent>,

    // Styling
    pub styles: AppStyles,
    pub theme: &'a Theme,
}

impl<'a> App<'a> {
    /// Constructs a new instance of [`App`].
    pub fn new(
        title: &'a str,
        config: &'a Config,
        theme: &'a Theme,
        state: &'a mut AppState<'a>,
    ) -> Result<Self> {
        Ok(Self {
            title,
            // State
            state,
            config,
            tick_rate: 100.0,
            frame_rate: 100.0,
            components: Vec::new(),
            current_screen: None,
            should_quit: false,
            should_suspend: false,
            screen: Screen::Start,
            last_tick_key_events: Vec::new(),

            // Styling
            styles: AppStyles {
                background: theme.main,
                popup_title: theme.popup_title,
                popup_content: theme.popup_fg_bg,
            },
            theme,
        })
    }

    pub fn register_component(&mut self, component: Rc<RefCell<impl Component + 'static>>) {
        self.components.push(component);
    }

    pub fn set_screen(&mut self, component: Rc<RefCell<impl Component + 'static>>) {
        self.current_screen = Some(component);
    }

    pub async fn run(&mut self) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = Tui::new(self.theme.main)?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);

        // tui.mouse(true);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component
                .borrow_mut()
                .register_action_handler(action_tx.clone())?;
        }

        for component in self.components.iter_mut() {
            component
                .borrow_mut()
                .register_config_handler(self.config.clone())?;
        }

        for component in self.components.iter_mut() {
            component.borrow_mut().init(tui.size()?)?;
        }

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    Event::Quit => action_tx.send(Action::Quit)?,
                    Event::Tick => action_tx.send(Action::Tick)?,
                    Event::Render => action_tx.send(Action::Render)?,
                    Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    Event::Key(key) => {
                        //log::info!("key: {key:?}");
                        #[allow(clippy::single_match)]
                        match key.code {
                            KeyCode::Char('q') => action_tx.send(Action::Quit)?,
                            _ => (),
                        }
                        /*if let Some(keymap) = self.config.keybindings.get(&self.mode) {
                        if let Some(action) = keymap.get(&vec![key]) {
                            log::info!("Got action: {action:?}");
                            action_tx.send(action.clone())?;
                        } else {
                            // If the key was not handled as a single key action,
                            // then consider it for multi-key combinations.
                            self.last_tick_key_events.push(key);

                            // Check for multi-key combinations
                            if let Some(action) = keymap.get(&self.last_tick_key_events) {
                            log::info!("Got action: {action:?}");
                            action_tx.send(action.clone())?;
                            }
                        }
                        };*/
                    }
                    _ => {}
                }

                for component in self.components.iter_mut() {
                    if let Some(action) = component.borrow_mut().handle_events(Some(e.clone()))? {
                        action_tx.send(action)?;
                    }
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    log::debug!("{action:?}");
                }

                match action {
                    Action::Tick => {
                        self.last_tick_key_events.drain(..);
                    }
                    Action::Quit => self.should_quit = true,
                    Action::Suspend => self.should_suspend = true,
                    Action::Resume => self.should_suspend = false,
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;

                        tui.draw(|f| {
                            // Set default theme for the frame.
                            let frame_size = f.size();
                            f.buffer_mut().set_style(frame_size, self.theme.main);

                            for component in self.components.iter_mut() {
                                let r = component.borrow_mut().draw(f, f.size(), self.theme);
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    Action::Render => {
                        tui.draw(|f| {
                            // Set default theme for the frame.
                            let frame_size = f.size();
                            f.buffer_mut().set_style(frame_size, self.theme.main);

                            for component in self.components.iter_mut() {
                                let r = component.borrow_mut().draw(f, f.size(), self.theme);
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    _ => {}
                }

                for component in self.components.iter_mut() {
                    if let Some(action) = component.borrow_mut().update(action.clone())? {
                        action_tx.send(action)?
                    };
                }
            }

            if self.should_suspend {
                tui.suspend()?;

                action_tx.send(Action::Resume)?;

                tui = Tui::new(self.theme.main)?
                    .tick_rate(self.tick_rate)
                    .frame_rate(self.frame_rate);
                // tui.mouse(true);

                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }

        tui.exit()?;
        Ok(())
    }
}

pub struct AppStyles {
    pub background: Style,
    pub popup_title: Style,
    pub popup_content: Style,
}
#[derive(Default)]
pub struct AppState<'data> {
    _lifetime: PhantomData<&'data ()>,
    pub running: bool,

    baseline_block_cursor: Option<&'data BlockCursor>,
}

impl<'data> AppState<'data> {
    pub fn new() -> Self {
        AppState {
            _lifetime: Default::default(),
            running: true,
            baseline_block_cursor: None,
        }
    }
}
