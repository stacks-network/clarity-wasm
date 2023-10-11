use anyhow::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("terminating gracefully")]
    Graceful(Error),
    
}

#[derive(Error, Debug)]
pub enum GracefulError {
    #[error("number of blocks processed has reached the specified maximum")]
    MaxProcessedBlockCountReached { processed_block_count: u32, max_blocks: u32 }
}