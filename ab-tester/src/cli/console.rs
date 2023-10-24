use crossterm::event::KeyEvent;
use ratatui::widgets::Widget;

pub mod app;
pub mod event;
pub mod handler;
pub mod tui;
pub mod theme;
mod ui;
mod screens;

mod widgets;
mod data;

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 15;

pub trait Screen: Widget {
    fn handle_key_event(&mut self, event: KeyEvent);
}