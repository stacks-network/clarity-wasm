use diesel::prelude::*;

use crate::schema::block_headers;

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, Associations, QueryableByName)]
#[diesel(primary_key(block_height))]
#[diesel(belongs_to(BlockHeader, foreign_key = parent_block_id))]
#[diesel(table_name = block_headers)]
pub struct BlockHeader {
    pub block_height: i32,
    pub index_block_hash: String,
    pub parent_block_id: String
}

impl BlockHeader {
    pub fn is_genesis(&self) -> bool {
        self.block_height == 0
    }
}