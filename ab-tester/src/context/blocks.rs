use std::fmt::Debug;
use std::path::Path;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use log::*;

use crate::stacks;
use crate::types::*;

/// Provides a cursor for navigating and iterating through [Block]s.
pub struct BlockCursor<'env> {
    height: usize,
    blocks_dir: &'env Path,
    headers: Vec<crate::types::BlockHeader>,
}

/// Manually implement [std::fmt::Debug] for [BlockCursor] since some fields
/// don't implement [std::fmt::Debug].
impl std::fmt::Debug for BlockCursor<'_> {
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
impl<'env> BlockCursor<'env> {
    /// Instantiates a new instance of [BlockCursor] using the provided `blocks_dir`
    /// and [BlockHeader]s.
    pub fn new(blocks_dir: &'env Path, headers: Vec<BlockHeader>) -> Self {
        Self {
            blocks_dir,
            height: 0,
            headers,
        }
    }

    pub fn len(&self) -> usize {
        self.headers.len()
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

        debug!("retrieving block with height {}", self.height);
        let block = self.get_block(height);

        self.height += 1;

        block
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
        debug!("loading block at height: {height}");
        if height >= self.headers.len() {
            debug!("> headers.len, returning None");
            return Ok(None);
        }

        let header = self.headers[height].clone();
        trace!("header: {header:?}");

        // Attempt to load the next (child) block header of this block. Note that
        // we can do this here because we know that we're using a canonical chain
        // with no forks. In the "real world" we would need to account for the
        // possibility of multiple children - one for each fork.
        let next_block_header = self.headers.get(height + 1).cloned();

        if height == 0 {
            debug!("returning genesis");
            return Ok(Some(Block::new_genesis(header.clone(), next_block_header)));
        }

        let parent_block_header = &self.headers[height - 1];

        // Get the block's path in chainstate.
        let block_id = header.index_block_hash;
        debug!("block_id: {block_id:?}");
        let block_path = stacks::StacksChainState::get_index_block_path(
            &self.blocks_dir.display().to_string(),
            &block_id,
        )?;
        debug!("block_path: {block_path:?}");
        // Load the block from chainstate.
        debug!("loading block with id {block_id} and path '{block_path}'");
        let stacks_block = stacks::StacksChainState::consensus_load(&block_path)?;
        trace!("block loaded: {stacks_block:?}");

        let block = Block::new_regular(
            header,
            stacks_block,
            next_block_header,
            parent_block_header.clone(),
        );

        Ok(Some(block))
    }
}

/// Provides an [Iterator] over blocks.
#[derive(Debug)]
pub struct BlockIntoIter<'env>(BlockCursor<'env>);

impl Iterator for BlockIntoIter<'_> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().expect("failed to retrieve next value")
    }
}

impl<'env> IntoIterator for BlockCursor<'env> {
    type Item = Block;

    type IntoIter = BlockIntoIter<'env>;

    fn into_iter(self) -> Self::IntoIter {
        BlockIntoIter(self)
    }
}

/// Representation of a Stacks block. Note that Box is used here to keep the
/// size of the enum on the stack in-check.
#[derive(Debug)]
pub enum Block {
    Genesis(Box<GenesisBlockInner>),
    Regular(Box<RegularBlockInner>),
}

#[derive(Debug)]
pub struct RegularBlockInner {
    pub header: BlockHeader,
    pub stacks_block: stacks::StacksBlock,
    pub parent_header: BlockHeader,
    pub next_header: Option<BlockHeader>,
}

#[derive(Debug)]
pub struct GenesisBlockInner {
    pub header: BlockHeader,
    pub next_header: Option<BlockHeader>,
}

/// Implementation of [Block] which provides various functions to consumers for
/// reading information about a Stacks block.
#[allow(dead_code)]
impl Block {
    /// Creates a new Regular block variant, i.e. not Boot or Genesis.
    pub fn new_regular(
        header: BlockHeader,
        stacks_block: stacks::StacksBlock,
        next_header: Option<BlockHeader>,
        parent_header: BlockHeader,
    ) -> Self {
        Block::Regular(Box::new(RegularBlockInner {
            header,
            stacks_block,
            next_header,
            parent_header,
        }))
    }

    /// Creates a new Genesis block variant. Genesis does not have a
    /// [stacks::StacksBlock] representation, so this function accepts only
    /// a [BlockHeader] to represent the block.
    pub fn new_genesis(header: BlockHeader, next_header: Option<BlockHeader>) -> Self {
        Block::Genesis(Box::new(GenesisBlockInner {
            header,
            next_header,
        }))
    }

    /// Gets the height for this block.
    pub fn block_height(&self) -> Result<u32> {
        let height = match self {
            Block::Genesis(_) => 0,
            Block::Regular(inner) => inner.header.block_height,
        };

        Ok(height)
    }

    /// Gets whether or not this block is the genesis (first) block in the
    /// chainstate. Note that the genesis block has special handling and is
    /// loaded from a pre-determined state (using `stx-genesis`) and does not have
    /// an index block hash in the MARFed chainstate either.
    pub fn is_genesis(&self) -> bool {
        if let Block::Genesis(_) = self {
            return true;
        }
        false
    }

    /// Retrieves the block hash as a [stacks::BlockHeaderHash] (backed by a
    /// `[u8;32]`). The boot block does not have a block hash and this function
    /// will return an error if you attempt to call it on the boot block. The
    /// genesis block statically returns the [stacks::FIRST_STACKS_BLOCK_HASH]
    /// constant.
    pub fn block_hash(&self) -> Result<stacks::BlockHeaderHash> {
        let hash = match self {
            Block::Genesis(_) => stacks::FIRST_STACKS_BLOCK_HASH,
            Block::Regular(inner) => inner.stacks_block.block_hash(),
        };

        Ok(hash)
    }

    /// Retrieves the index block hash, i.e. the MARF index hash for this block.
    /// Boot and Genesis blocks do not have index block hashes and this function
    /// will return an error if you attempt to call it on either of them.
    pub fn index_block_hash(&self) -> Result<&[u8]> {
        match self {
            Block::Genesis(inner) => Ok(inner.header.index_block_hash.as_bytes()),
            Block::Regular(inner) => Ok(inner.header.index_block_hash.as_bytes()),
        }
    }

    /// Retrieves this block's index block hash, i.e. the MARF index hash, as
    /// a [stacks::StacksBlockId].
    pub fn stacks_block_id(&self) -> Result<stacks::StacksBlockId> {
        let id = stacks::StacksBlockId::from_bytes(self.index_block_hash()?)
            .ok_or_else(|| anyhow!("failed to convert index block hash to stacks block id"))?;

        Ok(id)
    }
}
