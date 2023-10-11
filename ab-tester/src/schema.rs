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

    table! {
        marf_data (key_hash, block_height) {
            key_hash -> Binary,
            block_height -> Integer,
            block_hash -> Binary,
            contract_id -> Text,
            value -> Text
        }
    }
}
