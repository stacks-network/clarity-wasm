/// The purpose of this file is to re-export items from core Stacks crates
/// since we use a lot of similar naming. The convention is to add all usings
/// from Stacks libs (excluding `clarity` - see `clarity.rs`) to this file as
/// re-exports and qualifying all usings within this app with `stacks::`.

pub use blockstack_lib::{
    chainstate::stacks::{
        StacksBlock, 
        StacksBlockHeader,
        index::{MarfTrieId, TrieLeaf},
        index::node::{TrieNodeID, TrieNodeType, is_backptr, TriePath},
        index::storage::TrieStorageConnection,
        index::trie::Trie,
    },
    types::chainstate::StacksBlockId,
    types::StacksEpochId
};