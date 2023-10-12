pub mod chainstate_marf {
    use diesel::prelude::*;

    table! {
        block_headers (block_height) {
            block_height -> Integer,
            index_block_hash -> Text,
            parent_block_id -> Text
        }
    }
}

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
            stacks_block_id -> Integer,
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

    table! {
        contract_execution (id) {
            id -> Integer,
            block_id -> Integer,
            contract_id -> Integer,
            transaction_id -> Binary
        }
    }

    table! {
        contract_var_type (id) {
            id -> Integer,
            name -> Text
        }
    }

    table! {
        contract_var (id) {
            id -> Integer,
            key -> Text,
            contract_id -> Integer
        }
    }

    table! {
        contract_var_instance (id) {
            id -> Integer,
            contract_var_id -> Integer,
            block_id -> Integer,
            contract_execution_id -> Integer,
            value -> Binary
        }
    }

    table! {
        contract_map (id) {
            id -> Integer,
            name -> Text,
            contract_id -> Integer
        }
    }

    table! {
        contract_map_entry (id) {
            id -> Integer,
            contract_map_id -> Integer,
            block_id -> Integer,
            key_hash -> Binary,
            value -> Binary
        }
    }
}
