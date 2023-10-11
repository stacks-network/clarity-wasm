use diesel::prelude::*;

use crate::schema::{
    chainstate_marf::{block_headers}, 
    clarity_marf::{data_table, metadata_table}
};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(block_height))]
#[diesel(table_name = block_headers)]
pub struct BlockHeader {
    block_height: i32,
    pub index_block_hash: String,
    pub parent_block_id: String
}

impl BlockHeader {
    pub fn block_height(&self) -> u32 {
        self.block_height as u32
    }
}

impl BlockHeader {
    pub fn is_genesis(&self) -> bool {
        self.block_height == 0
    }
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(key))]
#[diesel(table_name = data_table)]
pub struct DataEntry {
    pub key: String,
    pub value: String
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(key, blockhash))]
#[diesel(table_name = metadata_table)]
pub struct MetaDataEntry {
    pub key: String,
    pub blockhash: String,
    pub value: String
}