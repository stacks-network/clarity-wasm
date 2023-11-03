/// Types used for reading from a chainstate MARF.
pub mod chainstate_db {
    use crate::schema::chainstate_marf::*;
    use diesel::prelude::*;

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(block_height))]
    #[diesel(table_name = block_headers)]
    pub struct BlockHeader {
        pub block_height: i32,
        pub index_block_hash: String,
        pub parent_block_id: String,
        pub consensus_hash: String,
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

/// Types used for reading from a chainstate Clarity MARF'd database.
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

/// Types for this application.
pub mod app_db {
    use crate::schema::appdb::*;
    use diesel::prelude::*;

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(table_name = runtime)]
    pub struct Runtime {
        pub id: i32,
        pub name: String,
    }

    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = environment)]
    pub struct Environment {
        pub id: i32,
        pub name: String,
        pub runtime_id: i32,
    }

    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = block)]
    pub struct Block {
        pub id: i32,
        pub environment_id: i32,
        //pub stacks_block_id: i32,
        pub height: i32,
        pub index_hash: Vec<u8>,
        pub marf_trie_root_hash: Vec<u8>,
    }

    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = marf_entry)]
    pub struct MarfEntry {
        pub id: i32,
        pub block_id: i32,
        pub key_hash: Vec<u8>,
        pub value: Vec<u8>,
    }

    /// A generalized instance to an installed Clarity contract.
    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = contract)]
    pub struct Contract {
        pub id: i32,
        pub block_id: i32,
        pub qualified_contract_id: String,
        pub source: Vec<u8>,
    }

    /// Holds information about a specific execution of a Clarity contract.
    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = contract_execution)]
    pub struct ContractExecution {
        pub id: i32,
        pub block_id: i32,
        pub contract_id: i32,
        pub transaction_id: Vec<u8>,
    }

    /// A data-var definition for a Clarity contract.
    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = contract_var)]
    pub struct ContractVar {
        pub id: i32,
        pub contract_id: i32,
        pub key: String,
    }

    /// A single Clarity data-var instance which is associated with a specific contract
    /// execution.
    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = contract_var_instance)]
    pub struct ContractVarInstance {
        pub id: i32,
        pub contract_var_id: i32,
        pub contract_execution_id: i32,
        pub value: Vec<u8>,
    }

    /// Information regarding Clarity maps in a contract.
    #[derive(
        Queryable,
        Selectable,
        Identifiable,
        PartialEq,
        Eq,
        Debug,
        Clone,
        QueryableByName,
        Insertable,
    )]
    #[diesel(table_name = contract_map)]
    pub struct ContractMap {
        pub id: i32,
        pub contract_id: i32,
        pub name: String,
    }
}
