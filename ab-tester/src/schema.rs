use diesel::prelude::*;

table! {
    block_headers (consensus_hash, block_hash) {
        consensus_hash -> Text,
        block_hash -> Text,
        index_block_hash -> Text,
        version -> Integer,
        parent_block -> Text,
        block_height -> Integer,
        parent_block_id -> Text,
    }
}

table! {
    marf_data (block_id) {
        block_id -> Integer,
        block_hash -> Text,
        //data -> Blob,
        unconfirmed -> Integer,
        external_offset -> Integer,
        external_length -> Integer
    }
}

//joinable!(marf_data -> block_headers (block_hash));
allow_tables_to_appear_in_same_query!(block_headers, marf_data,);