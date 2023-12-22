use diesel::prelude::*;

table! {
    block_headers (consensus_hash, block_hash) {
        version -> Integer,
        // Converted to/from u64.
        total_burn -> Text,
        // Converted to/from u64.
        total_work -> Text,
        proof -> Text,
        // Hash of parent Stacks block.
        parent_block -> Text,
        parent_microblock -> Text,
        parent_microblock_sequence -> Integer,
        tx_merkle_root -> Text,
        state_index_root -> Text,
        microblock_pubkey_hash -> Text,
        // Note: this is *not* unique, since two burn chain forks can commit
        // to the same Stacks block.
        block_hash -> Text,
        // Note: this is the hash of the block hash and consensus hash of the
        // burn block that selected it, and is guaranteed to be globally unique
        // (across all Stacks forks and across all PoX forks).
        // index_block_hash is the block hash fed into the MARF index.
        index_block_hash -> Text,
        block_height -> Integer,
        // Root hash of the internal, not-conensus-critical MARF that allows
        // us to track chainstate/fork metadata.
        index_root -> Text,
        // All consensus hashes are guaranteed to be unique.
        consensus_hash -> Text,
        // Burn header hash corresponding to the consensus hash (NOT guaranteed
        // to be unique, since we can have 2+ blocks per burn block if there's
        // a PoX fork).
        burn_header_hash -> Text,
        // Height of the burnchain block header that generated this consensus hash.
        burn_header_height -> Integer,
        // Timestamp from the burnchain block header that generated this consensus hash.
        burn_header_timestamp -> BigInt,
        // NOTE: this is the parent index_block_hash.
        parent_block_id -> Text,
        cost -> Text,
        // Converted to/from u64.
        block_size -> Text,
        affirmation_weight -> Integer
    }
}

table! {
    payments (address, block_hash) {
        address -> Text,
        block_hash -> Text,
        consensus_hash -> Text,
        parent_block_hash -> Text,
        parent_consensus_hash -> Text,
        coinbase -> Text,
        tx_fees_anchored -> Text,
        tx_fees_streamed -> Text,
        stx_burns -> Text,
        burnchain_commit_burn -> Integer,
        burnchain_sortition_burn -> Integer,
        miner -> Integer,
        stacks_block_height -> Integer,
        index_block_hash -> Text,
        vtxindex -> Integer,
        recipient -> Text,
    }
}

table! {
    // There are two rewards records per (parent,child) pair. One will have a
    // non-zero coinbase, the other will have a 0 coinbase.
    matured_rewards (parent_index_block_hash, child_index_block_hash, coinbase) {
        // Address of the miner who produced the block
        address -> Text,
        // Who received the reward (if different from the miner)
        recipient -> Text,
        // Will be 0 if this is the miner, >0 if this is a user burn support
        vtxindex -> Integer,
        coinbase -> Text,
        tx_fees_anchored -> Text,
        tx_fees_streamed_confirmed -> Text,
        tx_fees_streamed_produced -> Text,
        // Fork identifier (1)
        child_index_block_hash -> Text,
        // Fork identifier (2)
        parent_index_block_hash -> Text
    }
}

table! {
    db_config (version) {
        version -> Integer,
        mainnet -> Bool,
        chain_id -> Integer
    }
}
