/// Types used for reading from a chainstate MARF.
pub mod chainstate_db {
    use crate::db::schema::chainstate_marf::*;
    use diesel::prelude::*;

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(version))]
    #[diesel(table_name = db_config)]
    pub struct DbConfig {
        pub version: i32,
        pub mainnet: bool,
        pub chain_id: i32
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(parent_index_block_hash, child_index_block_hash, coinbase))]
    #[diesel(table_name = matured_rewards)]
    pub struct MaturedReward {
        pub address: String,
        pub recipient: String,
        pub vtxindex: i32,
        pub coinbase: String,
        pub tx_fees_anchored: String,
        pub tx_fees_streamed_confirmed: String,
        pub tx_fees_streamed_produced: String,
        pub child_index_block_hash: String,
        pub parent_index_block_hash: String,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(address, block_hash))]
    #[diesel(table_name = payments)]
    pub struct Payment {
        pub address: String,
        pub block_hash: String,
        pub burnchain_commit_burn: i32,
        pub burnchain_sortition_burn: i32,
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(consensus_hash, block_hash))]
    #[diesel(table_name = block_headers)]
    pub struct BlockHeader {
        pub version: i32,
        /// Converted to/from u64
        pub total_burn: String,
        /// Converted to/from u64
        pub total_work: String,
        pub proof: String,
        /// Hash of parent Stacks block.
        pub parent_block: String,
        pub parent_microblock: String,
        pub parent_microblock_sequence: i32,
        pub tx_merkle_root: String,
        pub state_index_root: String,
        pub microblock_pubkey_hash: String,
        /// Note: this is *not* unique, since two burn chain forks can commit
        /// to the same Stacks block.
        pub block_hash: String,
        /// Note: this is the hash of the block hash and consensus hash of the
        /// burn block that selected it, and is guaranteed to be globally unique
        /// (across all Stacks forks and across all PoX forks).
        /// index_block_hash is the block hash fed into the MARF index.
        pub index_block_hash: String,
        pub block_height: i32,
        /// Root hash of the internal, not-conensus-critical MARF that allows
        /// us to track chainstate/fork metadata.
        pub index_root: String,
        /// All consensus hashes are guaranteed to be unique.
        pub consensus_hash: String,
        /// Burn header hash corresponding to the consensus hash (NOT guaranteed
        /// to be unique, since we can have 2+ blocks per burn block if there's
        /// a PoX fork).
        pub burn_header_hash: String,
        /// Height of the burnchain block header that generated this consensus hash.
        pub burn_header_height: i32,
        /// Timestamp from the burnchain block header that generated this consensus hash.
        pub burn_header_timestamp: i64,
        /// NOTE: this is the parent index_block_hash.
        pub parent_block_id: String,
        pub cost: String,
        /// Converted to/from u64.
        pub block_size: String,
        pub affirmation_weight: i32,
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
    use crate::db::schema::clarity_marf::*;
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
    use crate::{clarity, db::schema::appdb::*, stacks, stacks::Address};
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

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(consensus_hash, block_hash))]
    #[diesel(table_name = _block_headers)]
    pub struct BlockHeader {
        pub version: i32,
        /// Converted to/from u64
        pub total_burn: i64,
        /// Converted to/from u64
        pub total_work: i64,
        pub proof: Vec<u8>,
        /// Hash of parent Stacks block.
        pub parent_block: Vec<u8>,
        pub parent_microblock: Vec<u8>,
        pub parent_microblock_sequence: i32,
        pub tx_merkle_root: Vec<u8>,
        pub state_index_root: Vec<u8>,
        pub microblock_pubkey_hash: Vec<u8>,
        /// Note: this is *not* unique, since two burn chain forks can commit
        /// to the same Stacks block.
        pub block_hash: Vec<u8>,
        /// Note: this is the hash of the block hash and consensus hash of the
        /// burn block that selected it, and is guaranteed to be globally unique
        /// (across all Stacks forks and across all PoX forks).
        /// index_block_hash is the block hash fed into the MARF index.
        pub index_block_hash: Vec<u8>,
        pub block_height: i32,
        /// Root hash of the internal, not-conensus-critical MARF that allows
        /// us to track chainstate/fork metadata.
        pub index_root: Vec<u8>,
        /// All consensus hashes are guaranteed to be unique.
        pub consensus_hash: Vec<u8>,
        /// Burn header hash corresponding to the consensus hash (NOT guaranteed
        /// to be unique, since we can have 2+ blocks per burn block if there's
        /// a PoX fork).
        pub burn_header_hash: Vec<u8>,
        /// Height of the burnchain block header that generated this consensus hash.
        pub burn_header_height: i32,
        /// Timestamp from the burnchain block header that generated this consensus hash.
        pub burn_header_timestamp: i64,
        /// NOTE: this is the parent index_block_hash.
        pub parent_block_id: Vec<u8>,
        pub cost: i64,
        /// Converted to/from u64.
        pub block_size: i64,
        pub affirmation_weight: i32,
    }

    /// Implement `From` for the `chainstate_db`'s model to keep the app code
    /// a little cleaner when importing from a Stacks node's db.
    impl From<super::chainstate_db::BlockHeader> for BlockHeader {
        fn from(value: super::chainstate_db::BlockHeader) -> Self {
            Self {
                version: value.version,
                total_burn: value
                    .total_burn
                    .parse()
                    .expect("failed to parse total_burn as u64"),
                total_work: value
                    .total_work
                    .parse()
                    .expect("failed to parse total_work as u64"),
                proof: hex::decode(value.proof).expect("failed to decode proof from hex"),
                parent_block: hex::decode(value.parent_block)
                    .expect("failed to decode parent_block from hex"),
                parent_microblock: hex::decode(value.parent_microblock)
                    .expect("failed to decode parent_microblock from hex"),
                parent_microblock_sequence: value.parent_microblock_sequence,
                tx_merkle_root: hex::decode(value.tx_merkle_root)
                    .expect("failed to decode tx_merkle_root from hex"),
                state_index_root: hex::decode(value.state_index_root)
                    .expect("failed to decode state_index_root from hex"),
                microblock_pubkey_hash: hex::decode(value.microblock_pubkey_hash)
                    .expect("failed to decode microblock_pubkey_hash from hex"),
                block_hash: hex::decode(value.block_hash)
                    .expect("failed to decode block_hash from hex"),
                index_block_hash: hex::decode(value.index_block_hash)
                    .expect("failed to decode index_block_hash from hex"),
                block_height: value.block_height,
                index_root: hex::decode(value.index_root)
                    .expect("failed to decode index_root from hex"),
                consensus_hash: hex::decode(value.consensus_hash)
                    .expect("failed to decode consensus_hash from hex"),
                burn_header_hash: hex::decode(value.burn_header_hash)
                    .expect("failed to decode burn_header_hash from hex"),
                burn_header_height: value.burn_header_height,
                burn_header_timestamp: value.burn_header_timestamp,
                parent_block_id: hex::decode(value.parent_block_id)
                    .expect("failed to decode parent_block_id from hex"),
                cost: value.cost.parse().expect("failed to parse cost as u64"),
                block_size: value
                    .block_size
                    .parse()
                    .expect("failed to parse block_size as u64"),
                affirmation_weight: value.affirmation_weight,
            }
        }
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(address, block_hash))]
    #[diesel(table_name = _payments)]
    pub struct Payment {
        pub address: String,
        pub block_hash: Vec<u8>,
        pub burnchain_commit_burn: i32,
        pub burnchain_sortition_burn: i32,
    }

    /// Implement `From` for the `chainstate_db`'s model to keep the app code
    /// a little cleaner when importing from a Stacks node's db.
    impl From<super::chainstate_db::Payment> for Payment {
        fn from(value: super::chainstate_db::Payment) -> Self {
            Payment {
                address: value.address,
                block_hash: hex::decode(value.block_hash)
                    .expect("failed to decode block_hash from hex"),
                burnchain_commit_burn: value.burnchain_commit_burn,
                burnchain_sortition_burn: value.burnchain_sortition_burn,
            }
        }
    }

    #[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
    #[diesel(primary_key(parent_index_block_hash, child_index_block_hash, coinbase))]
    #[diesel(table_name = _matured_rewards)]
    pub struct MaturedReward {
        pub address: String,
        pub recipient: String,
        pub vtxindex: i32,
        pub coinbase: i64,
        pub tx_fees_anchored: i32,
        pub tx_fees_streamed_confirmed: i32,
        pub tx_fees_streamed_produced: i32,
        pub child_index_block_hash: Vec<u8>,
        pub parent_index_block_hash: Vec<u8>,
    }

    impl Into<stacks::MinerReward> for &MaturedReward {
        fn into(self) -> stacks::MinerReward {
            stacks::MinerReward {
                address: stacks::StacksAddress::from_string(&self.address)
                    .expect("FATAL: could not parse miner address"),
                recipient: clarity::PrincipalData::parse(&self.recipient)
                    .expect("FATAL: could not parse recipient principal"),
                vtxindex: self.vtxindex as u32,
                coinbase: self.coinbase as u128,
                tx_fees_anchored: self.tx_fees_anchored as u128,
                tx_fees_streamed_confirmed: self.tx_fees_streamed_confirmed as u128,
                tx_fees_streamed_produced: self.tx_fees_streamed_produced as u128,
            }
        }
    }
}
