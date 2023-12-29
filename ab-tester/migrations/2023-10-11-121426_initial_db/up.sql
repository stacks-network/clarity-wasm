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
    network_id INTEGER NOT NULL,
    chain_id INTEGER NOT NULL,
    is_read_only BOOLEAN NOT NULL,
    environment_type_id INTEGER NOT NULL,
    last_block_height INTEGER NOT NULL,
    base_path TEXT NOT NULL,

    CONSTRAINT fk_runtime
    FOREIGN KEY (runtime_id)
    REFERENCES runtime (id)
);

CREATE TABLE IF NOT EXISTS environment_type (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

INSERT INTO environment_type VALUES (0, 'Stacks Node');
INSERT INTO environment_type VALUES (1, 'Network-Synced');
INSERT INTO environment_type VALUES (2, 'A/B Tester Instrumented');

CREATE TABLE IF NOT EXISTS environment_snapshot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    environment_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    block_height INTEGER NOT NULL,
    file_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS block (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    environment_id INTEGER NOT NULL,
    --stacks_block_id INTEGER NOT NULL UNIQUE,
    height INTEGER NOT NULL,
    index_hash BINARY NOT NULL,
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
    environment_id INTEGER NOT NULL,
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
    cost BINARY NOT NULL,
    block_size BIGINT NOT NULL,
    affirmation_weight INTEGER NOT NULL,

    PRIMARY KEY (environment_id, consensus_hash, block_hash),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE INDEX IF NOT EXISTS ix__block_headers_index_block_hash 
    ON _block_headers (environment_id, index_block_hash);

CREATE TABLE IF NOT EXISTS _payments (
    environment_id INTEGER NOT NULL,
    "address" TEXT NOT NULL,
    block_hash BINARY NOT NULL,
    consensus_hash BINARY NOT NULL,
    parent_block_hash BINARY NOT NULL,
    parent_consensus_hash BINARY NOT NULL,
    coinbase BIGINT NOT NULL,
    tx_fees_anchored BIGINT NOT NULL,
    tx_fees_streamed BIGINT NOT NULL,
    stx_burns BIGINT NOT NULL,
    burnchain_commit_burn INTEGER NOT NULL,
    burnchain_sortition_burn INTEGER NOT NULL,
    miner BOOLEAN NOT NULL,
    stacks_block_height INTEGER NOT NULL,
    index_block_hash BINARY NOT NULL,
    vtxindex INTEGER NOT NULL,
    recipient TEXT NOT NULL,

    PRIMARY KEY (environment_id, "address", block_hash),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE INDEX IF NOT EXISTS ix__payments_block_hash 
    ON _payments (environment_id, block_hash);

CREATE TABLE IF NOT EXISTS _matured_rewards (
    environment_id INTEGER NOT NULL,
    "address" TEXT NOT NULL,
    recipient TEXT NOT NULL,
    vtxindex INTEGER NOT NULL,
    coinbase BIGINT NOT NULL,
    tx_fees_anchored INTEGER NOT NULL,
    tx_fees_streamed_confirmed INTEGER NOT NULL,
    tx_fees_streamed_produced INTEGER NOT NULL,
    child_index_block_hash BINARY NOT NULL,
    parent_index_block_hash BINARY NOT NULL,

    PRIMARY KEY (environment_id, parent_index_block_hash, child_index_block_hash, coinbase),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE INDEX IF NOT EXISTS ix__matured_rewards_parent_child_index_block_hash 
    ON _matured_rewards (
        environment_id,
        parent_index_block_hash,
        child_index_block_hash
    );

CREATE TABLE IF NOT EXISTS _ast_rule_heights (
    environment_id INTEGER NOT NULL,
    ast_rule_id INTEGER NOT NULL,
    block_height INTEGER NOT NULL,

    PRIMARY KEY (environment_id, ast_rule_id),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE TABLE IF NOT EXISTS _epochs (
    environment_id INTEGER NOT NULL,
    start_block_height INTEGER NOT NULL,
    end_block_height INTEGER NOT NULL,
    epoch_id INTEGER NOT NULL,
    block_limit TEXT NOT NULL,
    network_epoch INTEGER NOT NULL,

    PRIMARY KEY (environment_id, start_block_height, epoch_id),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE TABLE IF NOT EXISTS _block_commits (
    environment_id INTEGER NOT NULL,
    txid BINARY NOT NULL,
    vtxindex INTEGER NOT NULL,
    block_height INTEGER NOT NULL,
    burn_header_hash BINARY NOT NULL,
    sortition_id BINARY NOT NULL,
    block_header_hash BINARY NOT NULL,
    new_seed BINARY NOT NULL,
    parent_block_ptr INTEGER NOT NULL,
    parent_vtxindex INTEGER NOT NULL,
    key_block_ptr INTEGER NOT NULL,
    key_vtxindex INTEGER NOT NULL,
    memo TEXT NOT NULL,
    commit_outs TEXT NOT NULL,
    burn_fee INTEGER NOT NULL,
    sunset_burn INTEGER NOT NULL,
    input TEXT NOT NULL,
    apparent_sender TEXT NOT NULL,
    burn_parent_modulus INTEGER NOT NULL,

    PRIMARY KEY (environment_id, txid, sortition_id),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE TABLE IF NOT EXISTS _snapshots (
    environment_id INTEGER NOT NULL,
    block_height INTEGER NOT NULL,
    burn_header_hash BINARY NOT NULL,
    sortition_id BINARY NOT NULL,
    parent_sortition_id BINARY NOT NULL,
    burn_header_timestamp INTEGER NOT NULL,
    parent_burn_header_hash BINARY NOT NULL,
    consensus_hash BINARY NOT NULL,
    ops_hash BINARY NOT NULL,
    total_burn INTEGER NOT NULL,
    sortition BOOLEAN NOT NULL,
    sortition_hash BINARY NOT NULL,
    winning_block_txid BINARY NOT NULL,
    winning_stacks_block_hash BINARY NOT NULL,
    index_root BINARY NOT NULL,
    num_sortitions INTEGER NOT NULL,
    stacks_block_accepted BOOLEAN NOT NULL,
    stacks_block_height INTEGER NOT NULL,
    arrival_index INTEGER NOT NULL,
    canonical_stacks_tip_height INTEGER NOT NULL,
    canonical_stacks_tip_hash BINARY NOT NULL,
    canonical_stacks_tip_consensus_hash BINARY NOT NULL,
    pox_valid BOOLEAN NOT NULL,
    accumulated_coinbase_ustx INTEGER NOT NULL,
    pox_payouts BINARY NOT NULL,

    PRIMARY KEY (environment_id, sortition_id),

    UNIQUE (environment_id, index_root),

    CONSTRAINT fk_environment
    FOREIGN KEY (environment_id)
    REFERENCES environment (id)
);

CREATE INDEX IF NOT EXISTS ix__snapshots_block_height 
    ON _snapshots (environment_id, block_height ASC);
CREATE INDEX IF NOT EXISTS ix__snapshots_consensus_hash 
    ON _snapshots (environment_id, consensus_hash);