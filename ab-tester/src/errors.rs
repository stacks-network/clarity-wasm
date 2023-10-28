use color_eyre::eyre::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError<'a> {
    #[error("terminating gracefully")]
    Graceful(&'a str),
}
