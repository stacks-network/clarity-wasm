CREATE TABLE IF NOT EXISTS runtime (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

INSERT INTO runtime VALUES (0, 'None (Read-Only)');
INSERT INTO runtime VALUES (1, 'Interpreter');
INSERT INTO runtime VALUES (2, 'Wasm');

CREATE TABLE IF NOT EXISTS environment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    runtime_id INTEGER NOT NULL,

    CONSTRAINT fk_runtime
    FOREIGN KEY (runtime_id)
    REFERENCES runtime (id)
);

INSERT INTO environment VALUES (1, 'baseline', 1);

CREATE TABLE IF NOT EXISTS block (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    environment_id INTEGER NOT NULL,
    --stacks_block_id INTEGER NOT NULL UNIQUE,
    height INTEGER UNIQUE NOT NULL,
    index_hash BINARY UNIQUE NOT NULL,
    marf_trie_root_hash BINARY NOT NULL,

    UNIQUE (environment_id, index_hash),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE TABLE IF NOT EXISTS marf_entry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER NOT NULL,
    key_hash BLOB NOT NULL,
    value BLOB NOT NULL,

    UNIQUE (block_id, key_hash),

    CONSTRAINT fk_block
    FOREIGN KEY (block_id)
    REFERENCES block (id)
);

CREATE TABLE IF NOT EXISTS contract (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER NOT NULL,
    qualified_contract_id TEXT NOT NULL,
    source BLOB NOT NULL,

    UNIQUE (qualified_contract_id),

    CONSTRAINT fk_block
    FOREIGN KEY (block_id)
    REFERENCES block (id)
);

CREATE TABLE IF NOT EXISTS contract_execution (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER NOT NULL,
    contract_id INTEGER NOT NULL,
    transaction_id BLOB NOT NULL,

    CONSTRAINT fk_block
    FOREIGN KEY (block_id)
    REFERENCES block (id),

    CONSTRAINT fk_contract
    FOREIGN KEY (contract_id)
    REFERENCES contract (id)
);

CREATE TABLE IF NOT EXISTS contract_var (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id INTEGER NOT NULL,
    "key" TEXT NOT NULL,

    UNIQUE (contract_id, "key"),

    CONSTRAINT fk_contract
    FOREIGN KEY (contract_id)
    REFERENCES contract (id)
);

CREATE TABLE IF NOT EXISTS contract_var_instance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_var_id INTEGER NOT NULL,
    block_id INTEGER NOT NULL,
    contract_execution_id INTEGER NOT NULL,
    value BLOB NOT NULL,

    CONSTRAINT fk_contract_var
    FOREIGN KEY (contract_var_id)
    REFERENCES contract_var (id),

    CONSTRAINT fk_block
    FOREIGN KEY (block_id)
    REFERENCES block (id)
);

CREATE TABLE IF NOT EXISTS contract_map (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id INTEGER NOT NULL,
    name TEXT NOT NULL,

    UNIQUE (contract_id, name),

    CONSTRAINT fk_contract
    FOREIGN KEY (contract_id)
    REFERENCES contract (id)
);

CREATE TABLE IF NOT EXISTS contract_map_entry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_map_id INTEGER NOT NULL,
    block_id INTEGER NOT NULL,
    key_hash BLOB NOT NULL,
    value BLOB NOT NULL,

    UNIQUE (contract_map_id, block_id, key_hash),

    CONSTRAINT fk_contract_map
    FOREIGN KEY (contract_map_id)
    REFERENCES contract_map (id),

    CONSTRAINT fk_block
    FOREIGN KEY (block_id)
    REFERENCES block (id)
);

CREATE TABLE IF NOT EXISTS _block_headers (
    "version" INTEGER NOT NULL,
    total_burn BIGINT NOT NULL,
    total_work BIGINT NOT NULL,
    proof BINARY NOT NULL,
    parent_block BINARY NOT NULL,
    parent_microblock BINARY NOT NULL,
    parent_microblock_sequence INTEGER NOT NULL,
    tx_merkle_root BINARY NOT NULL,
    state_index_root BINARY NOT NULL,
    microblock_pubkey_hash BINARY NOT NULL,
    block_hash BINARY NOT NULL,
    index_block_hash BINARY NOT NULL,
    block_height INTEGER NOT NULL,
    index_root BINARY NOT NULL,
    consensus_hash BINARY NOT NULL,
    burn_header_hash BINARY NOT NULL,
    burn_header_height INTEGER NOT NULL,
    burn_header_timestamp BIGINT NOT NULL,
    parent_block_id BINARY NOT NULL,
    cost BIGINT NOT NULL,
    block_size BIGINT NOT NULL,
    affirmation_weight INTEGER NOT NULL,

    PRIMARY KEY (consensus_hash, block_hash)
);

CREATE TABLE IF NOT EXISTS _payments (
    "address" TEXT NOT NULL,
    block_hash BINARY NOT NULL,
    burnchain_commit_burn INTEGER NOT NULL,
    burnchain_sortition_burn INTEGER NOT NULL,

    PRIMARY KEY ("address", block_hash)
);

CREATE TABLE IF NOT EXISTS _matured_rewards (
    "address" TEXT NOT NULL,
    recipient TEXT NOT NULL,
    vtxindex INTEGER NOT NULL,
    coinbase BIGINT NOT NULL,
    tx_fees_anchored INTEGER NOT NULL,
    tx_fees_streamed_confirmed INTEGER NOT NULL,
    tx_fees_streamed_produced INTEGER NOT NULL,
    child_index_block_hash BINARY NOT NULL,
    parent_index_block_hash BINARY NOT NULL,

    PRIMARY KEY (parent_index_block_hash, child_index_block_hash, coinbase)
);
