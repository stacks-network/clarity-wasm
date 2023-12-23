use std::cell::RefCell;
use std::path::Path;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel::{debug_query, insert_into, OptionalExtension, QueryDsl, SqliteConnection};
use log::*;

use super::model::chainstate::{BlockHeader, MaturedReward, Payment};
use super::schema::chainstate::*;
use crate::{clarity, stacks};

pub struct StacksHeadersDb {
    conn: RefCell<SqliteConnection>,
}

impl StacksHeadersDb {
    pub fn new(index_db_path: &Path) -> Result<Self> {
        Ok(Self {
            conn: RefCell::new(SqliteConnection::establish(
                &index_db_path.display().to_string(),
            )?),
        })
    }

    pub fn import_block_headers(
        &mut self,
        headers: Box<dyn Iterator<Item = Result<crate::types::BlockHeader>>>,
        _environment_id: Option<i32>, // Not supported for stacks node environments
    ) -> Result<()> {
        let conn = &mut *self.conn.borrow_mut();

        conn.transaction(|tx| -> Result<()> {
            for header in headers {
                let header = header
                    .map_err(|e| error!("{:?}", e))
                    .expect("failed to load header");

                trace!(
                    "inserting block header {{hash: {:?}, height: {:?}}}",
                    &header.block_hash,
                    &header.block_height
                );
                let header: super::model::chainstate::BlockHeader = header.try_into()?;

                let insert_stmt = insert_into(block_headers::table)
                    .values(header)
                    .on_conflict((
                        block_headers::consensus_hash,
                        block_headers::block_hash
                    ))
                    .do_update()
                    .set((
                        block_headers::consensus_hash.eq(excluded(block_headers::consensus_hash)),
                        block_headers::block_hash.eq(excluded(block_headers::block_hash))
                    ));

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one block header, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    pub fn import_payments(
        &mut self,
        payments: Box<dyn Iterator<Item = Result<crate::types::Payment>>>,
        _environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn.borrow_mut();

        conn.transaction(|tx| -> Result<()> {
            for payment in payments {
                let payment = payment
                    .map_err(|e| error!("{:?}", e))
                    .expect("failed to load payment");

                trace!("inserting payment {{address:  {:?}, block hash: {:?}}}", 
                    &payment.address,
                    &payment.block_hash);

                let payment: super::model::chainstate::Payment = payment.try_into()?;

                let insert_stmt = insert_into(payments::table)
                    .values(payment)
                    .on_conflict_do_nothing();

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one payment, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    /// Attempts to fetch a [BlockHeader] from the database using its
    /// [stacks::StacksBlockId]. If no records are found, this function will return
    /// [None], and will panic if the query fails to execute.
    fn get_block_header_by_stacks_block_id(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Result<Option<BlockHeader>> {
        let result = block_headers::table
            .filter(block_headers::index_block_hash.eq(id_bhh.to_hex()))
            .first::<BlockHeader>(&mut *self.conn.borrow_mut())
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
        let result = payments::table
            .filter(payments::block_hash.eq(id_bhh.to_hex()))
            .first::<Payment>(&mut *self.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

        Ok(result)
    }
}

impl clarity::HeadersDB for StacksHeadersDb {
    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Option<stacks::BlockHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::BlockHeaderHash(
                    hex::decode(header.index_block_hash)
                        .expect("failed to decode index block hash hex")
                        .try_into()
                        .expect("failed to convert index block hash into a 32-byte array"),
                )
            })
    }

    fn get_burn_header_hash_for_block(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Option<stacks::BurnchainHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::BurnchainHeaderHash(
                    hex::decode(header.burn_header_hash)
                        .expect("failed to decode burn header hash hex")
                        .try_into()
                        .expect("failed to convert burn header hash into a 32-byte array"),
                )
            })
    }

    fn get_consensus_hash_for_block(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Option<stacks::ConsensusHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::ConsensusHash(
                    hex::decode(header.consensus_hash)
                        .expect("failed to decode consensus hash hex")
                        .try_into()
                        .expect("failed to convert consensus hash into a 20-byte array"),
                )
            })
    }

    fn get_vrf_seed_for_block(&self, id_bhh: &stacks::StacksBlockId) -> Option<stacks::VRFSeed> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| {
                stacks::VRFSeed(
                    hex::decode(header.proof)
                        .expect("failed to decode proof/vrf seed hex")
                        .try_into()
                        .expect("failed to convert the VRF seed (proof) into a 32-byte array"),
                )
            })
    }

    fn get_burn_block_time_for_block(&self, id_bhh: &stacks::StacksBlockId) -> Option<u64> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| header.burn_header_timestamp as u64)
    }

    fn get_burn_block_height_for_block(&self, id_bhh: &stacks::StacksBlockId) -> Option<u32> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| header.burn_header_height as u32)
    }

    fn get_miner_address(&self, id_bhh: &stacks::StacksBlockId) -> Option<stacks::StacksAddress> {
        let result = payments::table
            .filter(payments::index_block_hash.eq(id_bhh.to_hex()))
            .first::<Payment>(&mut *self.conn.borrow_mut())
            .optional()
            .expect("sql query execution failed")
            .map(|payment| {
                stacks::Address::from_string(&payment.address)
                    .expect("failed to convert the payment address to a StacksAddress")
            });
        
        result
    }

    fn get_burnchain_tokens_spent_for_block(&self, id_bhh: &stacks::StacksBlockId) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_sortition_burn as u128)
    }

    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_commit_burn as u128)
    }

    fn get_tokens_earned_for_block(&self, child_id_bhh: &stacks::StacksBlockId) -> Option<u128> {
        let parent_id_bhh = self
            .get_block_header_by_stacks_block_id(child_id_bhh)
            .unwrap()
            .map(|header| header.parent_block_id)
            .expect("failed to find parent block header for child block");

        let rewards = matured_rewards::table
            .filter(
                matured_rewards::parent_index_block_hash
                    .eq(parent_id_bhh)
                    .and(matured_rewards::child_index_block_hash.eq(child_id_bhh.to_hex())),
            )
            .get_results::<MaturedReward>(&mut *self.conn.borrow_mut())
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
