use diesel::prelude::*;

// Defines the available runtimes.
table! {
    runtime (id) {
        id -> Integer,
        name -> Text
    }
}

// Defines the available runtime environments. Environment `1` is always
// baseline. The remaining environments are defined by the user.
table! {
    environment (id) {
        id -> Integer,
        name -> Text,
        runtime_id -> Integer,
        path -> Text,
    }
}

// Holds information about Stacks blocks in each environment.
table! {
    block (id) {
        // NOTE: This is an _internal_ id, see `stacks_block_id` for the
        // id of the block according to the blockchain.
        id -> Integer,
        environment_id -> Integer,
        //stacks_block_id -> Integer,
        height -> Integer,
        index_hash -> Binary,
        marf_trie_root_hash -> Binary
    }
}

// Holds information about the marf
table! {
    marf_entry (id) {
        id -> Integer,
        block_id -> Integer,
        key_hash -> Binary,
        value -> Binary
    }
}

// Holds information about Clarity contracts in each environment.
table! {
    contract (id) {
        id -> Integer,
        block_id -> Integer,
        qualified_contract_id -> Text,
        source -> Binary
    }
}

// Contains information about persisted variables for each contract in the
// baseline environment.
table! {
    contract_var (id) {
        id -> Integer,
        key -> Text,
        contract_id -> Integer
    }
}

// Contains information about maps for each contract in the baseline environment.
table! {
    contract_map (id) {
        id -> Integer,
        name -> Text,
        contract_id -> Integer
    }
}

// Contains a log of all contract executions, both from baseline and for
// any additional executions in other environments.
table! {
    contract_execution (id) {
        id -> Integer,
        block_id -> Integer,
        contract_id -> Integer,
        transaction_id -> Binary
    }
}

// Contains a changelog of contract variables across executions.
table! {
    contract_var_instance (id) {
        id -> Integer,
        contract_var_id -> Integer,
        contract_execution_id -> Integer,
        value -> Binary
    }
}

// Don't think we can actually implement this...
table! {
    contract_map_entry (id) {
        id -> Integer,
        contract_map_id -> Integer,
        block_id -> Integer,
        key_hash -> Binary,
        value -> Binary
    }
}

// Table which represents the MARF index database's `block_headers` table.
// Note that in this table schema we are using more optimized storage using
// BigInts and Binary (blob) fields instead of strings/hex strings.
table! {
    _block_headers (environment_id, consensus_hash, block_hash) {
        environment_id -> Integer,
        version -> Integer,
        // Converted to/from u64.
        total_burn -> BigInt,
        // Converted to/from u64.
        total_work -> BigInt,
        proof -> Binary,
        // Hash of parent Stacks block.
        parent_block -> Binary,
        parent_microblock -> Binary,
        parent_microblock_sequence -> Integer,
        tx_merkle_root -> Binary,
        state_index_root -> Binary,
        microblock_pubkey_hash -> Binary,
        // Note: this is *not* unique, since two burn chain forks can commit
        // to the same Stacks block.
        block_hash -> Binary,
        // Note: this is the hash of the block hash and consensus hash of the
        // burn block that selected it, and is guaranteed to be globally unique
        // (across all Stacks forks and across all PoX forks).
        // index_block_hash is the block hash fed into the MARF index.
        index_block_hash -> Binary,
        block_height -> Integer,
        // Root hash of the internal, not-conensus-critical MARF that allows
        // us to track chainstate/fork metadata.
        index_root -> Binary,
        // All consensus hashes are guaranteed to be unique.
        consensus_hash -> Binary,
        // Burn header hash corresponding to the consensus hash (NOT guaranteed
        // to be unique, since we can have 2+ blocks per burn block if there's
        // a PoX fork).
        burn_header_hash -> Binary,
        // Height of the burnchain block header that generated this consensus hash.
        burn_header_height -> Integer,
        // Timestamp from the burnchain block header that generated this consensus hash.
        burn_header_timestamp -> BigInt,
        // NOTE: this is the parent index_block_hash.
        parent_block_id -> Binary,
        cost -> Binary,
        // Converted to/from u64.
        block_size -> BigInt,
        affirmation_weight -> Integer,
    }
}

// Table which represents the MARF index database's `payments` table. Note
// that in this table schema we are using more optimized storage using Binary
// (blob) fields instead of hex strings. This schema also only includes a
// subset of the fields from the actual MARF index as we do not need all of
// the original fields for replaying transactions.
table! {
    _payments (environment_id, address, block_hash) {
        environment_id -> Integer,
        address -> Text,
        block_hash -> Binary,
        burnchain_commit_burn -> Integer,
        burnchain_sortition_burn -> Integer,
    }
}

// Table which represents the MARF index database's `matured_rewards` table.
// Note that in this table schema we are using more optimized storage using
// Binary (blob) fields instead of hex strings and Integer/BigInt instead of
// Text fields.
table! {
    _matured_rewards (environment_id, parent_index_block_hash, child_index_block_hash, coinbase) {
        environment_id -> Integer,
        address -> Text,
        recipient -> Text,
        vtxindex -> Integer,
        coinbase -> BigInt,
        tx_fees_anchored -> Integer,
        tx_fees_streamed_confirmed -> Integer,
        tx_fees_streamed_produced -> Integer,
        child_index_block_hash -> Binary,
        parent_index_block_hash -> Binary
    }
}

// Table which represents the sortition DB's `ast_rule_heights` table.
table! {
    _ast_rule_heights (environment_id, ast_rule_id) {
        environment_id -> Integer,
        ast_rule_id -> Integer,
        block_height -> Integer
    }
}

// Table which represents the sortition DB's `epochs` table.
table! {
    _epochs (environment_id, start_block_height, epoch_id) {
        environment_id -> Integer,
        start_block_height -> Integer,
        end_block_height -> Integer,
        epoch_id -> Integer,
        block_limit -> Binary,
        network_epoch -> Integer
    }
}

// Table which represents the sortition DB's `block_commits` table. Note that
// in this table schema we are using optimized column types (i.e. Binary to
// represent hex instead of Text).
table! {
    _block_commits (environment_id, txid, sortition_id) {
        environment_id -> Integer,
        txid -> Binary,
        vtxindex -> Integer,
        block_height -> Integer,
        burn_header_hash -> Binary,
        sortition_id -> Binary,
        block_header_hash -> Binary,
        new_seed -> Binary,
        parent_block_ptr -> Integer,
        parent_vtxindex -> Integer,
        key_block_ptr -> Integer,
        key_vtxindex -> Integer,
        memo -> Text,
        commit_outs -> Binary,
        burn_fee -> BigInt,
        sunset_burn -> BigInt,
        input -> Binary,
        apparent_sender -> Binary,
        burn_parent_modulus -> Integer
    }
}

table! {
    _snapshots (environment_id, sortition_id) {
        environment_id -> Integer,
        block_height -> Integer,
        burn_header_hash -> Binary,
        sortition_id -> Binary,
        parent_sortition_id -> Binary,
        burn_header_timestamp -> BigInt,
        parent_burn_header_hash -> Binary,
        consensus_hash -> Binary,
        ops_hash -> Binary,
        total_burn -> BigInt,
        sortition -> Bool,
        sortition_hash -> Binary,
        winning_block_txid -> Binary,
        winning_stacks_block_hash -> Binary,
        index_root -> Binary,
        num_sortitions -> Integer,
        stacks_block_accepted -> Bool,
        stacks_block_height -> Integer,
        arrival_index -> Integer,
        canonical_stacks_tip_height -> Integer,
        canonical_stacks_tip_hash -> Binary,
        canonical_stacks_tip_consensus_hash -> Binary,
        pox_valid -> Bool,
        accumulated_coinbase_ustx -> BigInt,
        pox_payouts -> Binary
    }
}
