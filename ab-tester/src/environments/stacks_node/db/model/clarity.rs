/// This file contains model types (DTOs) which represent tables in the RDBMS storage
/// for a Stacks node's Clarity DB, typically located in `chainstate/vm/clarity/marf.sqlite`.
use diesel::prelude::*;

use super::super::schema::clarity::*;

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(key))]
#[diesel(table_name = data_table)]
pub struct DataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(key, blockhash))]
#[diesel(table_name = metadata_table)]
pub struct MetaDataEntry {
    pub key: String,
    pub blockhash: String,
    pub value: String,
}
