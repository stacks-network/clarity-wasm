use diesel::prelude::*;

use crate::schema::block_headers;

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone)]
#[diesel(primary_key(block_height))]
#[diesel(table_name = block_headers)]
pub struct BlockHeader {
    pub block_height: i32,
    pub index_block_hash: String,
    pub parent_block_id: String
}