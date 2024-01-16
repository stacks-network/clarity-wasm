use color_eyre::Result;
use diesel::{QueryDsl, RunQueryDsl};

use crate::{
    environments::{
        BoxedDbIterResult, stacks_node::db::{schema::chainstate::block_headers, stacks_headers_db::StacksHeadersDb}, ReadableEnv
    }, 
    db::{
        model, 
        schema::{_payments, _snapshots, _block_commits}
    }, 
    context::BlockCursor
};

use super::InstrumentedEnv;

/// Implementation of [ReadableEnv] for [InstrumentedEnv].
impl ReadableEnv for InstrumentedEnv {
    fn blocks(&self, max_blocks: Option<u32>) -> Result<BlockCursor> {
        let headers = self.block_headers(max_blocks)?;
        let cursor = BlockCursor::new(self.env_config.paths.blocks_dir(), headers);
        Ok(cursor)
    }

    fn last_block_height(&self) -> Result<u32> {
        self.app_db.last_block_height(self.id)
    }

    fn snapshots(&self, prefetch_limit: u32) -> BoxedDbIterResult<crate::types::Snapshot> {
        let result = self
            .app_db
            .stream_results::<model::Snapshot, crate::types::Snapshot, _>(
                _snapshots::table, 
                prefetch_limit as usize
            );

        Ok(Box::new(result))
    }

    fn snapshot_count(&self) -> Result<usize> {
        self.app_db.snapshot_count(self.id)
    }

    fn block_commits(&self, _prefetch_limit: u32) -> Result<Box<dyn Iterator<Item = Result<crate::types::BlockCommit>>>> {
        let result = self
            .app_db
            .stream_results::<model::BlockCommit, crate::types::BlockCommit, _>(
                _block_commits::table,
                1000,
            );

        Ok(Box::new(result))
    }

    fn block_commit_count(&self) -> Result<usize> {
        self.app_db.block_commit_count(self.id)
    }

    fn ast_rules(&self) -> BoxedDbIterResult<crate::types::AstRuleHeight> {
        todo!()
    }

    fn ast_rule_count(&self) -> Result<usize> {
        self.app_db.ast_rule_count(self.id)
    }

    fn epochs(&self) -> BoxedDbIterResult<crate::types::Epoch> {
        todo!()
    }

    fn epoch_count(&self) -> Result<usize> {
        self.app_db.epoch_count(self.id)
    }

    fn block_headers(&self, _prefetch_limit: u32) -> BoxedDbIterResult<crate::types::BlockHeader> {
        todo!()
    }

    fn block_header_count(&self) -> Result<usize> {
        let result: i64 = block_headers::table
            .count()
            .get_result(&mut *self.get_env_state()?.index_db_conn.borrow_mut())?;

        Ok(result as usize)
    }

    fn payments(&self, prefetch_limit: u32) -> BoxedDbIterResult<crate::types::Payment> {
        let result = self
            .app_db
            .stream_results::<model::Payment, crate::types::Payment, _>(
                _payments::table,
                prefetch_limit as usize,
            );

        Ok(Box::new(result))
    }

    fn payment_count(&self) -> Result<usize> {
        let headers_db = StacksHeadersDb::new(self.env_config.paths.index_db_path())?;
        let count = headers_db.payments_count()?;
        Ok(count as usize)
    }
}