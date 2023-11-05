/// Tables from the chainstate Sqlite database.
pub mod chainstate_marf {
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
        }
    }
}

/// Tables from the Clarity Sqlite database.
pub mod clarity_marf {
    use diesel::prelude::*;

    table! {
        data_table (key) {
            key -> Text,
            value -> Text
        }
    }

    table! {
        metadata_table (key, blockhash) {
            key -> Text,
            blockhash -> Text,
            value -> Text
        }
    }
}

/// Tables for this application.
pub mod appdb {
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
            runtime_id -> Integer
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

    table! {
        _block_headers (consensus_hash, block_hash) {
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
            cost -> BigInt,
            // Converted to/from u64.
            block_size -> BigInt,
            affirmation_weight -> Integer,
        }
    }
}
