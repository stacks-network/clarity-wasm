use diesel::prelude::*;

table! {
    block_headers (block_height) {
        block_height -> Integer,
        index_block_hash -> Text,
        parent_block_id -> Text
    }
}
