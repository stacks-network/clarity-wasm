use diesel::prelude::*;

table! {
    block_headers (block_height) {
        block_height -> Integer,
        index_block_hash -> Text,
        parent_block_id -> Text
    }
}

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