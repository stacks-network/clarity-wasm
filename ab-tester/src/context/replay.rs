use color_eyre::{eyre::ensure, Result};
use log::*;

use crate::{context::{Block, environments::RuntimeEnv}, errors::AppError, ok};

use super::{
    callbacks::ReplayCallbacks,
    environments::{ReadableEnv, WriteableEnv, AsRuntimeEnv},
};

/// Options for replaying an environment's chain into another environment.
#[derive(Default, Clone)]
pub struct ReplayOpts<'a> {
    pub from_height: Option<u32>,
    pub to_height: Option<u32>,
    pub max_blocks: Option<u32>,
    pub callbacks: ReplayCallbacks<'a>,
}

/// Validation/assertion helper methods for [ReplayOpts].
impl<'a> ReplayOpts<'a> {
    pub fn with_callbacks(&'_ mut self, callbacks: ReplayCallbacks<'a>) {
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
        source: &'a dyn ReadableEnv<'a>,
        target: &'a mut dyn WriteableEnv<'a>,
        opts: &'a ReplayOpts<'a>,
    ) -> Result<()> {
        info!(
            "aggregating contract calls starting at block height {}...",
            opts.from_height.unwrap_or(0)
        );

        let mut processed_block_count = 0;

        let blocks = source.blocks()?;
        (opts.callbacks.replay_start)(source.as_env(), target.as_env(), blocks.len());

        for block in source.blocks()?.into_iter() {
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

            processed_block_count += 1;
            continue;

            /*let block_id = header.stacks_block_id()?;

            for tx in stacks_block.unwrap().txs.iter() {
                info!("processing tx: {}", tx.txid());

                let origin_principal = StandardPrincipalData::from(tx.origin_address());

                #[allow(clippy::single_match)]
                match &tx.payload {
                    TransactionPayload::SmartContract(contract, _) => {
                        let contract_id =
                            QualifiedContractIdentifier::new(origin_principal, contract.name.clone());

                        if let Some(entry) = contracts.get(&contract_id) {
                            warn!(
                                "duplicate: {}, first block={}, second block={}",
                                contract_id, entry, &block_id
                            );
                        } else {
                            contracts.insert(contract_id, block_id);
                        }
                    }
                    _ => {}
                }
            }*/
        }

        (opts.callbacks.replay_finish)(source.as_env(), target.as_env());
        info!("blocks processed: {processed_block_count}");

        ok!()
    }
}

pub struct ReplayResult {}
