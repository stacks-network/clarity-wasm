pub mod chainstate_db {
    use crate::schema::chainstate_marf::*;
    use diesel::prelude::*;

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(block_height))]
    #[diesel(table_name = block_headers)]
    pub struct BlockHeader {
        block_height: i32,
        pub index_block_hash: String,
        pub parent_block_id: String,
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
}

pub mod clarity_db {
    use crate::schema::clarity_marf::*;
    use diesel::prelude::*;

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
}

pub mod app_db {
    use crate::schema::appdb::*;
    use diesel::prelude::*;

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = runtime)]
    pub struct Runtime {
        pub id: i32,
        name: String,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = environment)]
    pub struct Environment {
        id: i32,
        name: String,
        runtime_id: i32,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = block)]
    pub struct Block {
        id: i32,
        environment_id: i32,
        stacks_block_id: i32,
        height: i32,
        index_hash: Vec<u8>,
        marf_trie_root_hash: Vec<u8>,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = marf_entry)]
    pub struct MarfEntry {
        id: i32,
        block_id: i32,
        key_hash: Vec<u8>,
        value: Vec<u8>,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = contract)]
    pub struct Contract {
        id: i32,
        block_id: i32,
        qualified_contract_id: String,
        source: Vec<u8>,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = contract_execution)]
    pub struct ContractExecution {
        id: i32,
        block_id: i32,
        contract_id: i32,
        transaction_id: Vec<u8>,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = contract_var)]
    pub struct ContractVar {
        id: i32,
        contract_id: i32,
        key: String,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = contract_var_instance)]
    pub struct ContractVarInstance {
        id: i32,
        contract_var_id: i32,
        block_id: i32,
        contract_execution_id: i32,
        value: Vec<u8>,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = contract_map)]
    pub struct ContractMap {
        id: i32,
        contract_id: i32,
        name: String,
    }
}
