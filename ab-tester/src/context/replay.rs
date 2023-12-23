use color_eyre::eyre::ensure;
use color_eyre::Result;
use log::*;

use super::callbacks::ReplayCallbackHandler;
use crate::context::Block;
use crate::environments::{ReadableEnv, WriteableEnv};
use crate::errors::AppError;
use crate::ok;

/// Options for replaying an environment's chain into another environment.
pub struct ReplayOpts<C>
where
    C: ReplayCallbackHandler,
{
    pub from_height: Option<u32>,
    pub to_height: Option<u32>,
    pub max_blocks: Option<u32>,
    pub callbacks: C,
    pub working_dir: String,
    pub snapshot_restore: bool
}

impl<C> Default for ReplayOpts<C>
where
    C: ReplayCallbackHandler + Default,
{
    fn default() -> Self {
        Self {
            from_height: Default::default(),
            to_height: Default::default(),
            max_blocks: Default::default(),
            callbacks: C::default(),
            working_dir: Default::default(),
            snapshot_restore: false
        }
    }
}

/// Validation/assertion helper methods for [ReplayOpts].
impl<C: ReplayCallbackHandler> ReplayOpts<C> {
    pub fn with_working_dir(&mut self, working_dir: &str) -> &mut Self {
        self.working_dir = working_dir.to_string();
        self
    }
    pub fn with_callbacks(&mut self, callbacks: C) -> &mut Self {
        self.callbacks = callbacks;
        self
    }

    pub fn build(self) -> Self {
        self
    }

    /// Asserts that the current `processeed_block_count` hasn't exceedeed the
    /// provided block count parameter.
    pub fn assert_max_processed_block_count(&self, processed_block_count: u32) -> Result<()> {
        if let Some(max_blocks) = self.max_blocks {
            ensure!(
                processed_block_count < max_blocks,
                AppError::Graceful("number of blocks processed has reached the specified maximum")
            );
        }

        ok!()
    }

    /// Asserts that the provided block height doesn't exceed the max block height,
    /// if provided.
    pub fn assert_block_height_under_max_height(&self, block_height: u32) -> Result<()> {
        if let Some(to_height) = self.to_height {
            ensure!(
                block_height <= to_height,
                AppError::Graceful(
                    "block height has reached the specified maximum block height (to-height)"
                )
            )
        }

        ok!()
    }
}

/// Provides methods for replaying a [ReadableEnv] into a [WriteableEnv].
pub struct ChainStateReplayer {}

impl ChainStateReplayer {
    pub fn replay<C: ReplayCallbackHandler>(
        source: &dyn ReadableEnv,
        target: &mut dyn WriteableEnv,
        opts: &ReplayOpts<C>,
    ) -> Result<()> {
        info!(
            "aggregating contract calls starting at block height {}...",
            opts.from_height.unwrap_or(0)
        );

        let mut processed_block_count = 0;

        let blocks = source.blocks(opts.max_blocks)?;
        opts.callbacks.replay_start(source, target, blocks.len());

        for block in blocks.into_iter() {
            opts.callbacks
                .replay_block_start(source, target, block.block_height()?);

            let (header, stacks_block) = match &block {
                Block::Genesis(inner) => {
                    // We can't process genesis (doesn't exist in chainstate), so skip it.
                    //info!("genesis block - skipping '{:?}'", gen.index_block_hash);
                    //continue;
                    info!("genesis block: '{:?}'", inner.header.index_block_hash);
                    (inner.header.clone(), None)
                }
                Block::Regular(inner) => (inner.header.clone(), Some(inner.stacks_block.clone())),
            };

            // Ensure that we've reached the specified block-height before beginning
            // processing.
            if header.block_height < opts.from_height.unwrap_or(0) {
                continue;
            }

            // Ensure that we haven't exceeded the specified max-blocks for processing.
            opts.assert_max_processed_block_count(processed_block_count)?;

            // Ensure that we haven't reached the specified max block-height for processing.
            opts.assert_block_height_under_max_height(header.block_height)?;

            if stacks_block.is_some() {
                info!(
                    "processing REGULAR block #{} ({})",
                    header.block_height,
                    hex::encode(header.index_block_hash)
                );
            } else {
                info!(
                    "processing GENESIS block #{} ({})",
                    header.block_height,
                    hex::encode(header.index_block_hash)
                );
            }

            // Process the block in the target environment.
            target.process_block(&block)?;

            opts.callbacks.replay_block_finish(source, target);
            processed_block_count += 1;
        }

        opts.callbacks.replay_finish(source, target);
        info!("blocks processed: {processed_block_count}");

        ok!()
    }
}

pub struct ReplayResult {}

impl ReplayResult {
    pub fn do_nothing(&self) {}
}