use color_eyre::eyre::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("terminating gracefully")]
    Graceful(Error),
}
