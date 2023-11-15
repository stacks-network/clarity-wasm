use color_eyre::eyre::ensure;
use color_eyre::Result;
use log::*;

use super::callbacks::{DefaultReplayCallbacks, ReplayCallbackHandler};
use super::environments::{ReadableEnv, RuntimeEnvContext, RuntimeEnvContextMut};
use crate::context::Block;
use crate::errors::AppError;
use crate::{clarity, ok, stacks};

/// Options for replaying an environment's chain into another environment.
pub struct ReplayOpts {
    pub from_height: Option<u32>,
    pub to_height: Option<u32>,
    pub max_blocks: Option<u32>,
    pub callbacks: Box<dyn ReplayCallbackHandler>,
}

impl Default for ReplayOpts {
    fn default() -> Self {
        Self {
            from_height: Default::default(),
            to_height: Default::default(),
            max_blocks: Default::default(),
            callbacks: Box::<DefaultReplayCallbacks>::default(),
        }
    }
}

/// Validation/assertion helper methods for [ReplayOpts].
impl ReplayOpts {
    pub fn with_callbacks(&mut self, callbacks: Box<dyn ReplayCallbackHandler>) {
        self.callbacks = callbacks;
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
    pub fn replay<'a>(
        source: &'a RuntimeEnvContext,
        target: &'a RuntimeEnvContextMut,
        opts: &'a ReplayOpts,
    ) -> Result<()> {
        info!(
            "aggregating contract calls starting at block height {}...",
            opts.from_height.unwrap_or(0)
        );

        let mut processed_block_count = 0;

        let blocks = source.blocks()?;
        opts.callbacks.replay_start(source, target, blocks.len());

        for block in source.blocks()?.into_iter() {
            opts.callbacks
                .replay_block_start(source, target, block.block_height()?);
            let (header, stacks_block) = match &block {
                Block::Boot(header) => {
                    // We can't process the boot block, so skip it.
                    info!("boot block - skipping '{:?}'", header.index_block_hash);
                    continue;
                }
                Block::Genesis(header) => {
                    // We can't process genesis (doesn't exist in chainstate), so skip it.
                    //info!("genesis block - skipping '{:?}'", gen.index_block_hash);
                    //continue;
                    info!("genesis block");
                    (header, None)
                }
                Block::Regular(header, block) => (header, Some(block)),
            };

            // Ensure that we've reached the specified block-height before beginning
            // processing.
            if header.block_height() < opts.from_height.unwrap_or(0) {
                continue;
            }

            // Ensure that we haven't exceeded the specified max-blocks for processing.
            opts.assert_max_processed_block_count(processed_block_count)?;

            // Ensure that we haven't reached the specified max block-height for processing.
            opts.assert_block_height_under_max_height(header.block_height())?;

            debug!(
                "processing block #{} ({})",
                header.block_height(),
                &hex::encode(&header.index_block_hash)
            );

            /*replay_env_mut.block_begin(&block, |_ctx| {
                info!("processing block!");
                ok!()
            })?;*/

            let block_id = header.stacks_block_id()?;

            for tx in stacks_block.unwrap().txs.iter() {
                info!("processing tx: {}", tx.txid());

                let origin_principal = clarity::StandardPrincipalData::from(tx.origin_address());

                #[allow(clippy::single_match)]
                match &tx.payload {
                    stacks::TransactionPayload::SmartContract(contract, _) => {
                        let contract_id = clarity::QualifiedContractIdentifier::new(
                            origin_principal,
                            contract.name.clone(),
                        );

                        info!("block id: {block_id:?}, contract id: {contract_id:?}");
                    }
                    _ => {}
                }
            }

            opts.callbacks.replay_block_finish(source, target);
            processed_block_count += 1;
        }

        opts.callbacks.replay_finish(source, target);
        info!("blocks processed: {processed_block_count}");

        ok!()
    }
}

pub struct ReplayResult {}
