use diesel::prelude::*;

use crate::schema::{block_headers, marf_data};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone)]
#[diesel(primary_key(consensus_hash, block_hash))]
#[diesel(table_name = block_headers)]
pub struct BlockHeader {
    consensus_hash: String,
    block_hash: String,
    index_block_hash: String,
    version: i32,
    parent_block: String,
    block_height: u32,
    parent_block_id: u32,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone, Associations)]
#[diesel(primary_key(block_id))]
#[diesel(belongs_to(BlockHeader, foreign_key = block_hash))]
#[diesel(table_name = marf_data)]
pub struct MarfData {
    block_id: u32,
    block_hash: String,
    unconfirmed: i32,
    external_offset: i32,
    external_length: i32
}

/*
block_id -> Integer,
        block_hash -> Text,
        data -> Blob,
        unconfirmed -> Integer,
        external_offset -> Integer,
        external_length -> Integer */