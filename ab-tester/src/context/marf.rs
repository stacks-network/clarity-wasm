use color_eyre::Result;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection};
use log::*;

use crate::{
    clarity,
    db::{model, schema},
    stacks,
};

struct MarfWalker {}

impl MarfWalker {
    /// Loads the specified block from the MARF.
    pub fn load_block(
        chainstate: &mut stacks::StacksChainState,
        clarity_db_conn: &mut SqliteConnection,
        block_id: &stacks::StacksBlockId,
    ) -> Result<()> {
        debug!("beginning to walk the block: {}", block_id);
        let leaves = Self::walk_block(chainstate, block_id, false)?;

        if !leaves.is_empty() {
            debug!("finished walking, leaf count: {}", leaves.len());
        } else {
            warn!("no leaves found");
        }

        for leaf in leaves {
            let value = schema::clarity_marf::data_table::table
                .filter(schema::clarity_marf::data_table::key.eq(leaf.data.to_string()))
                .first::<model::clarity_db::DataEntry>(clarity_db_conn)
                .optional()?;

            if let Some(value_unwrapped) = value {
                let clarity_value =
                    clarity::Value::try_deserialize_hex_untyped(&value_unwrapped.value);
                if let Ok(clarity_value) = clarity_value {
                    trace!("deserialized value: {:?}", &clarity_value);
                } else {
                    debug!("failed to deserialize value: {:?}", &value_unwrapped.value);
                }
            }
        }

        Ok(())
    }

    /// Helper function for [`Self::load_block()`] which is used to walk the MARF,
    /// looking for leaf nodes.
    ///
    /// If `follow_backptrs` is true, the entire MARF from genesis _up to and
    /// including the specified `block_id`_ will be read. At higher blocks heights this
    /// is very slow.
    fn walk_block(
        chainstate: &mut stacks::StacksChainState,
        block_id: &stacks::StacksBlockId,
        follow_backptrs: bool,
    ) -> Result<Vec<stacks::TrieLeaf>> {
        use stacks::*;

        let mut leaves: Vec<TrieLeaf> = Default::default();

        chainstate.with_clarity_marf(|marf| -> Result<()> {
            let mut marf = marf.reopen_readonly()?;
            let _root_hash = marf.get_root_hash_at(block_id)?;

            let _ = marf.with_conn(|storage| -> Result<()> {
                debug!("opening block {block_id}");
                storage.open_block(block_id)?;
                let (root_node_type, _) = Trie::read_root(storage)?;

                let mut level: u32 = 0;
                Self::inner_walk_block(
                    storage,
                    &root_node_type,
                    &mut level,
                    follow_backptrs,
                    &mut leaves,
                )?;

                Ok(())
            });
            Ok(())
        })?;

        Ok(leaves)
    }

    /// Helper function for [`Self::walk_block()`] which is used for recursion
    /// through the [MARF](blockstack_lib::chainstate::stacks::index::MARF).
    fn inner_walk_block<T: stacks::MarfTrieId>(
        storage: &mut stacks::TrieStorageConnection<T>,
        node: &stacks::TrieNodeType,
        level: &mut u32,
        follow_backptrs: bool,
        leaves: &mut Vec<stacks::TrieLeaf>,
    ) -> Result<()> {
        use stacks::*;

        *level += 1;
        let node_type_id = TrieNodeID::from_u8(node.id()).unwrap();
        debug!(
            "[level {level}] processing {node_type_id:?} with {} ptrs",
            &node.ptrs().len()
        );

        match &node {
            TrieNodeType::Leaf(leaf) => {
                leaves.push(leaf.clone());
                *level -= 1;
                trace!("[level {level}] returned to level");
                return Ok(());
            }
            _ => {
                let mut ptr_number = 0;
                for ptr in node.ptrs().iter() {
                    ptr_number += 1;
                    trace!("[level {level}] [ptr no. {ptr_number}] ptr: {ptr:?}");

                    if is_backptr(ptr.id) {
                        if !follow_backptrs {
                            continue;
                        }
                        // Handle back-pointers

                        // Snapshot the current block hash & id so that we can rollback
                        // to them after we're finished processing this back-pointer.
                        let (current_block, current_id) = storage.get_cur_block_and_id();

                        // Get the block hash for the block the back-pointer is pointing to
                        let back_block_hash =
                            storage.get_block_from_local_id(ptr.back_block())?.clone();

                        trace!("[level {level}] following backptr: {ptr:?}, {back_block_hash}");

                        // Open the block to which the back-pointer is pointing.
                        storage.open_block_known_id(&back_block_hash, ptr.back_block())?;

                        // Read the back-pointer type.
                        let backptr_node_type =
                            storage.read_nodetype_nohash(&ptr.from_backptr())?;

                        // Walk the newly opened block using the back-pointer.
                        Self::inner_walk_block(
                            storage,
                            &backptr_node_type,
                            level,
                            follow_backptrs,
                            leaves,
                        )?;

                        // Return to the previous block
                        trace!(
                            "[level {level}] returning to context: {current_block} {current_id:?}"
                        );
                        storage.open_block_known_id(&current_block, current_id.unwrap())?;
                    } else {
                        trace!("[level {level}] following normal ptr: {ptr:?}");
                        // Snapshot the current block hash & id so that we can rollback
                        // to them after we're finished processing this back-pointer.
                        let (current_block, current_id) = storage.get_cur_block_and_id();
                        trace!(
                            "[level {level}] current block: {} :: {current_block}",
                            current_id.unwrap()
                        );

                        // Handle nodes contained within this block/trie
                        trace!("hello");
                        let type_id = TrieNodeID::from_u8(ptr.id()).unwrap();
                        if type_id == TrieNodeID::Empty {
                            trace!("[level {level}] reached empty node, continuing");
                            continue;
                        }

                        trace!("[level {level}] ptr node type: {type_id:?}");
                        let node_type = storage.read_nodetype_nohash(ptr).unwrap();

                        trace!(
                            "[level {level}] {:?} => {ptr:?}, ptrs: {}",
                            TrieNodeID::from_u8(ptr.id()),
                            node_type.ptrs().len()
                        );
                        Self::inner_walk_block(
                            storage,
                            &node_type,
                            level,
                            follow_backptrs,
                            leaves,
                        )?;
                    }
                }
            }
        }

        *level -= 1;
        trace!("[level {level}] returned to level");
        Ok(())
    }

    /// Loads the block with the specified block hash from chainstate (the `blocks`
    /// directory for the node).
    pub fn get_stacks_block(blocks_dir: &str, block_hash: &str) -> Result<stacks::StacksBlock> {
        let block_id = stacks::StacksBlockId::from_hex(block_hash)?;
        let block_path = stacks::StacksChainState::get_index_block_path(blocks_dir, &block_id)?;
        let block = stacks::StacksChainState::consensus_load(&block_path)?;

        Ok(block)
    }
}
