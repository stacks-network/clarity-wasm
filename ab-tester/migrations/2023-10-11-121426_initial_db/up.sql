BEGIN;

CREATE TABLE IF NOT EXISTS runtime (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS block (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    environment_id INTEGER NOT NULL,
    stacks_block_id INTEGER NOT NULL UNIQUE,
    height INTEGER UNIQUE NOT NULL,
    index_hash BINARY UNIQUE NOT NULL,
    marf_trie_root_hash BINARY NOT NULL,

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
    qualified_contract_id TEXT NOT NULL UNIQUE,
    source BLOB NOT NULL,

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

COMMIT;
