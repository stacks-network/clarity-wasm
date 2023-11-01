use blockstack_lib::chainstate::stacks::{db::StacksChainState, StacksBlock};
use color_eyre::{Result, eyre::bail};
use stacks_common::types::chainstate::StacksBlockId;
use log::*;

use crate::{model::chainstate_db, stacks};

/// Provides a cursor for navigating and iterating through [Block]s.
pub struct BlockCursor {
    height: usize,
    blocks_dir: String,
    headers: Vec<chainstate_db::BlockHeader>,
}

/// Manually implement [std::fmt::Debug] for [BlockCursor] since some fields
/// don't implement [std::fmt::Debug].
impl std::fmt::Debug for BlockCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockCursor")
            .field("pos", &self.height)
            .field("conn", &"...")
            .field("headers", &self.headers)
            .finish()
    }
}

/// Implementation of [BlockCursor].
#[allow(dead_code)]
impl BlockCursor {
    pub fn new(blocks_dir: &str, headers: Vec<chainstate_db::BlockHeader>) -> Self {
        Self {
            blocks_dir: blocks_dir.to_string(),
            height: 0,
            headers,
        }
    }

    /// Gets the current position of the cursor.
    pub fn pos(&self) -> usize {
        self.height
    }

    /// Moves the cursor to the specified position.
    pub fn seek(&mut self, height: usize) -> Result<&mut Self> {
        if height >= self.headers.len() {
            bail!("Attempted to seek beyond chain tip");
        }
        self.height = height;
        Ok(self)
    }

    /// Retrieves the next [Block] relative the current block height and
    /// increments the cursor position.
    pub fn next(&mut self) -> Result<Option<Block>> {
        let height = self.height;

        if height >= self.headers.len() {
            return Ok(None);
        }

        self.height += 1;
        info!("retrieving block with height {}", self.height);
        self.get_block(height)
    }

    /// Decrements the cursor position and retrieves the [Block] at the
    /// decremented position (current position - 1). Returns [None] if there is
    /// no previous block (current block height is zero).
    pub fn prev(&mut self) -> Result<Option<Block>> {
        if self.height == 0 {
            return Ok(None);
        }

        self.height -= 1;
        let block = self.get_block(self.height)?;
        Ok(block)
    }

    /// Retrieves the [Block] at the specified height without moving the cursor.
    /// Returns [None] if there is no [Block] at the specified height.
    pub fn peek(&mut self, height: usize) -> Result<Option<Block>> {
        self.get_block(height)
    }

    /// Retrieves the next [Block] without moving the cursor position. If the
    /// next height exceeds the chain tip this function will return [None].
    pub fn peek_next(&mut self) -> Result<Option<Block>> {
        let block = self.get_block(self.height + 1)?;
        Ok(block)
    }

    /// Retrieves the previous [Block] in relation to the current block height
    /// without moving the cursor position. If there is no previous block (the
    /// current height is zero) then this function will return [None].
    pub fn peek_prev(&mut self) -> Result<Option<Block>> {
        let block = self.get_block(self.height - 1)?;
        Ok(block)
    }

    /// Loads the block with the specified block hash from chainstate (the `blocks`
    /// directory for the node).
    fn get_block(&self, height: usize) -> Result<Option<Block>> {
        info!("loading block at height: {height}");
        if height >= self.headers.len() {
            info!("> headers.len, returning None");
            return Ok(None);
        }

        let header = self.headers[height].clone();
        info!("header: {header:?}");

        if height == 0 {
            info!("returning genesis");
            Block::new_genesis(header.clone());
        }

        // Get the block's path in chainstate.
        let block_id = StacksBlockId::from_hex(&header.index_block_hash)?;
        info!("block_id: {block_id:?}");
        let block_path = StacksChainState::get_index_block_path(&self.blocks_dir, &block_id)?;
        info!("block_path: {block_path:?}");
        // Load the block from chainstate.
        info!("loading block with id {block_id} and path '{block_path}'");
        let stacks_block = StacksChainState::consensus_load(&block_path)?;
        info!("block loaded: {stacks_block:?}");

        let block = Block::new(header, stacks_block);

        Ok(Some(block))
    }
}

/// Provides an [Iterator] over blocks.
pub struct BlockIntoIter(BlockCursor);

impl Iterator for BlockIntoIter {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.0.next()
            .expect("failed to retrieve next value");
        next
    }
}

impl IntoIterator for BlockCursor {
    type Item = Block;

    type IntoIter = BlockIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        BlockIntoIter(self)
    }
}

pub enum Block {
    Genesis(chainstate_db::BlockHeader),
    Regular(chainstate_db::BlockHeader, stacks::StacksBlock)
}

/// Implementation of [Block] which provides various functions to consumers for
/// reading information about a Stacks block.
#[allow(dead_code)]
impl Block {
    pub fn new(header: chainstate_db::BlockHeader, block: StacksBlock) -> Self {
        Block::Regular(header, block)
    }

    pub fn new_genesis(header: chainstate_db::BlockHeader) -> Self {
        Block::Genesis(header)
    }

    pub fn block_height(&self) -> u32 {
        match self {
            Block::Genesis(_) => 0,
            Block::Regular(header, _) => header.block_height()
        }
    }

    pub fn is_genesis(&self) -> bool {
        if let Block::Genesis(_) = self {
            return true;
        }
        return false;
    }

    pub fn index_block_hash(&self) -> Result<&str> {
        match self {
            Block::Genesis(_) => bail!("genesis block does not have an index block hash"),
            Block::Regular(header, _) => Ok(&header.index_block_hash)
        }
    }
}