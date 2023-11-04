/// The purpose of this file is to re-export items from core Stacks crates
/// since we use a lot of similar naming. The convention is to add all usings
/// from Stacks libs (excluding `clarity` - see `clarity.rs`) to this file as
/// re-exports and qualifying all usings within this app with `stacks::`.

pub use blockstack_lib::{
    burnchains::{Burnchain, PoxConstants},
    chainstate::burn::db::sortdb::{SortitionDB, SortitionDBTxContext},
    chainstate::stacks::{
        db::{
            ChainStateBootData, ChainstateAccountBalance, ChainstateAccountLockup,
            ChainstateBNSName, ChainstateBNSNamespace, ChainstateTx, StacksChainState,
        },
        index::marf::{MARFOpenOpts, MarfConnection},
        index::node::{is_backptr, TrieNodeID, TrieNodeType, TriePath},
        index::storage::TrieStorageConnection,
        index::trie::Trie,
        index::{ClarityMarfTrieId, MarfTrieId, TrieLeaf},
        StacksBlock, StacksBlockHeader, StacksTransaction, TransactionPayload,
    },
    clarity_vm::clarity::{ClarityBlockConnection, ClarityInstance, ClarityTransactionConnection},
    core::{
        BITCOIN_MAINNET_FIRST_BLOCK_HASH, BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
        BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP, FIRST_STACKS_BLOCK_HASH, STACKS_EPOCHS_MAINNET,
    },
    types::StacksEpoch,
    util_lib::db::IndexDBConn,
};

pub use stacks_common::types::{
    chainstate::{
        BlockHeaderHash, BurnchainHeaderHash, ConsensusHash, SortitionId, 
        StacksBlockId, VRFSeed
    },
    StacksEpochId,
};

pub use stx_genesis::*;
