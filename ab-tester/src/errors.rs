use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError<'a> {
    #[error("terminating gracefully")]
    Graceful(&'a str),
}
