pub mod blocks;
pub mod main;
pub mod start;

pub use {blocks::BlocksScreen, start::StartScreen};

pub enum Screen {
    Start,
}