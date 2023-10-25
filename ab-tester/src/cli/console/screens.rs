pub mod blocks;
pub mod start;
pub mod main;

pub use {
    blocks::BlocksScreen,
    start::StartScreen
};

pub enum Screen {
    Start
}