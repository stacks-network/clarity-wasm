use anyhow::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("terminating gracefully")]
    Graceful(Error),
}
