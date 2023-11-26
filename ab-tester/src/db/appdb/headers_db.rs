use std::rc::Rc;

use color_eyre::Result;
use color_eyre::eyre::anyhow;
use diesel::prelude::*;
use diesel::QueryDsl;

use crate::{
    clarity,
    db::{
        model::app_db::{BlockHeader, MaturedReward, Payment},
        schema::appdb::*,
    },
    stacks,
};

use super::AppDb;

pub trait AsHeadersDb {
    fn as_headers_db(&self) -> &dyn clarity::HeadersDB;
}

pub struct AppDbHeadersWrapper {
    environment_id: i32,
    app_db: Rc<AppDb>
}

impl AsHeadersDb for AppDbHeadersWrapper {
    fn as_headers_db(&self) -> &dyn clarity::HeadersDB {
        self as &dyn clarity::HeadersDB
    }
}

impl AppDbHeadersWrapper {
    pub fn new(environment_id: i32, app_db: Rc<AppDb>) -> Self {
        Self {
            environment_id,
            app_db
        }
    }

    /// Attempts to fetch a [BlockHeader] from the database using its
    /// [stacks::StacksBlockId]. If no records are found, this function will return
    /// [None], and will panic if the query fails to execute.
    fn get_block_header_by_stacks_block_id(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Result<Option<BlockHeader>> {
        let result = _block_headers::table
            .filter(
                _block_headers::environment_id.eq(self.environment_id)
                .and(_block_headers::index_block_hash.eq(id_bhh.as_bytes().to_vec()))
            )
            .first::<BlockHeader>(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

        Ok(result)
    }

    /// Attempts to fetch a [Payment] from the database using its [stacks::StacksBlockId].
    /// If no records are found, this function will return [None], and will panic
    /// if the query fails to execute.
    fn get_payment_by_stacks_block_id(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Result<Option<Payment>> {
        let result = _payments::table
            .filter(
                _payments::environment_id.eq(self.environment_id)
                .and(_payments::block_hash.eq(id_bhh.as_bytes().to_vec()))
            )
            .first::<Payment>(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

        Ok(result)
    }
}

/// Implementation of Clarity's [clarity::HeadersDB] for the app datastore.
impl clarity::HeadersDB for AppDbHeadersWrapper {
    /// Retrieves the [stacks::BlockHeaderHash] for the Stacks block header with the
    /// given index block hash.
    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::BlockHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::BlockHeaderHash(
                    header
                        .index_block_hash
                        .try_into()
                        .expect("failed to convert index block hash into a 32-byte array"),
                )
            })
    }

    /// Retrieves the [stacks::BurnchainHeaderHash] for the Stacks block header
    /// with the given index block hash.
    fn get_burn_header_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::BurnchainHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::BurnchainHeaderHash(
                    header
                        .burn_header_hash
                        .try_into()
                        .expect("failed to convert burn header hash into a 32-byte array"),
                )
            })
    }

    /// Retrieves the [stacks::ConsensusHash] for the Stacks block header with
    /// the given index block hash.
    fn get_consensus_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::ConsensusHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::ConsensusHash(
                    header
                        .consensus_hash
                        .try_into()
                        .expect("failed to convert consensus hash into a 20-byte array"),
                )
            })
    }

    /// Retrieves the [stacks::VRFSeed] (proof) for the Stacks block header with
    /// the given index block hash.
    fn get_vrf_seed_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::VRFSeed> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::VRFSeed(
                    header
                        .proof
                        .try_into()
                        .expect("failed to convert the VRF seed (proof) into a 32-byte array"),
                )
            })
    }

    /// Retrieves the burn block timestamp as a [u64] for the Stacks block header
    /// with the given index block hash.
    fn get_burn_block_time_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u64> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| header.burn_header_timestamp as u64)
    }

    /// Retrieves the block height of the associated burn bunrh as a [u32] for
    /// the Stacks block header with the given index block hash.
    fn get_burn_block_height_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u32> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| header.burn_header_height as u32)
    }

    /// Attempts to retrieve the [stacks::StacksAddress] of the miner who mined
    /// the specified [stacks::StacksBlockId]. Returns [None] if a [Payment] entry
    /// for the specified block could not be found.
    fn get_miner_address(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::StacksAddress> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| {
                stacks::Address::from_string(&payment.address)
                    .expect("failed to convert the payment address to a StacksAddress")
            })
    }

    /// Attempts to retrieve the number of burnchain tokens spent for mining the
    /// specified [stacks::StacksBlockId] (`payments.burnchain_sortition_burn`).
    /// Returns [None] if a [Payment] entry for the specified block could not be found.
    /// TODO: Ensure that this description is correct.
    fn get_burnchain_tokens_spent_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_sortition_burn as u128)
    }

    /// Attempts to retrieve the number of burnchain tokens (i.e. BTC) spent for
    /// winning the specified [stacks::StacksBlockId] (`payments.burnchain_commit_burn`).
    /// Returns [None] if a [Payment] entry for the specified block could not be found.
    /// TODO: Ensure that this description is correct.
    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_commit_burn as u128)
    }

    /// Attempts to retrieve the number of tokens (STX) which were earned for
    /// the specified [stacks::StacksBlockId].
    /// TODO: This method currently panics if
    /// anything isn't correct - this could be improved.
    /// TODO: Ensure that this description is correct.
    fn get_tokens_earned_for_block(
        &self,
        child_id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        let parent_id_bhh = self
            .get_block_header_by_stacks_block_id(child_id_bhh)
            .unwrap()
            .map(|header| header.parent_block_id)
            .expect("failed to find parent block header for child block");

        let rewards = _matured_rewards::table
            .filter(
                _matured_rewards::environment_id.eq(self.environment_id)
                .and(_matured_rewards::parent_index_block_hash.eq(parent_id_bhh))
                .and(_matured_rewards::child_index_block_hash.eq(child_id_bhh.as_bytes().to_vec())),
            )
            .get_results::<MaturedReward>(&mut *self.app_db.conn.borrow_mut())
            .expect("failed to find matured rewards for parent+child block combination")
            .iter()
            .map(|result| result.into())
            .collect::<Vec<stacks::MinerReward>>();

        let reward = if rewards.len() == 2 {
            Some(if rewards[0].is_child() {
                rewards[0]
                    .try_add_parent(&rewards[1])
                    .expect("FATAL: got two child rewards")
            } else if rewards[1].is_child() {
                rewards[1]
                    .try_add_parent(&rewards[0])
                    .expect("FATAL: got two child rewards")
            } else {
                panic!("FATAL: got two parent rewards");
            })
        } else if child_id_bhh
            == &stacks::StacksBlockHeader::make_index_block_hash(
                &stacks::FIRST_BURNCHAIN_CONSENSUS_HASH,
                &stacks::FIRST_STACKS_BLOCK_HASH,
            )
        {
            Some(stacks::MinerReward::genesis(true)) //TODO: get this value from env
        } else {
            None
        };

        reward.and_then(|reward| reward.total().into())
    }
}