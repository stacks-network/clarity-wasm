use diesel::{SqliteConnection, connection::SimpleConnection};
use crate::Result;

pub fn install_clarity_db_instrumentation(conn: &mut SqliteConnection) -> Result<()> {
    conn.batch_execute(CLARITY_DB_INSTRUMENTATION_SQL)?;
    Ok(())
}

pub const CLARITY_DB_INSTRUMENTATION_SQL: &str = r#"
-- ----------------------------------------
-- # metadata_table
-- ----------------------------------------
DROP TABLE IF EXISTS tmp_metadata_table;

CREATE TABLE tmp_metadata_table (
    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
    is_update BOOLEAN NOT NULL,
    key TEXT NOT NULL,
    blockhash TEXT,
    value TEXT
);

DROP TRIGGER IF EXISTS insert_into_metadata_table;

CREATE TRIGGER insert_into_metadata_table 
AFTER INSERT ON metadata_table 
FOR EACH ROW 
BEGIN
    INSERT INTO
        tmp_metadata_table
        (
            is_update,
            key,
            blockhash,
            value
        )
        VALUES
        (
            0,
            new.key, 
            new.blockhash, 
            new.value
        );
END;

DROP TRIGGER IF EXISTS update_metadata_table;

CREATE TRIGGER update_metadata_table 
AFTER UPDATE ON metadata_table 
FOR EACH ROW 
BEGIN
    INSERT INTO
        tmp_metadata_table
        (
            is_update,
            key,
            blockhash,
            value
        )
        VALUES
        (
            1,
            new.key, 
            new.blockhash, 
            new.value
        );
END;

-- ----------------------------------------
-- # data_table
-- ----------------------------------------
DROP TABLE IF EXISTS tmp_data_table;

CREATE TABLE tmp_data_table (
    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
    is_update BOOLEAN NOT NULL,
    key TEXT,
    value TEXT
  );

DROP TRIGGER IF EXISTS insert_into_data_table;

CREATE TRIGGER insert_into_data_table 
AFTER INSERT ON data_table 
FOR EACH ROW 
BEGIN
    INSERT INTO
        tmp_data_table (is_update, key, value)
        VALUES (0, new.key, new.value)
    ;
END;

DROP TRIGGER IF EXISTS update_data_table;

CREATE TRIGGER update_data_table 
AFTER UPDATE ON data_table 
FOR EACH ROW 
BEGIN
    INSERT INTO
        tmp_data_table (is_update, key, value)
        VALUES (1, new.key, new.value)
    ;
END;

-- ----------------------------------------
-- # marf_data
-- ----------------------------------------
DROP TABLE IF EXISTS tmp_marf_data;

CREATE TABLE tmp_marf_data (
   row_id INTEGER PRIMARY KEY AUTOINCREMENT,
   is_update BOOLEAN NOT NULL,
   block_id INTEGER, 
   block_hash TEXT NOT NULL,
   data BLOB NOT NULL,
   unconfirmed INTEGER NOT NULL,
   external_offset INTEGER DEFAULT 0 NOT NULL,
   external_length INTEGER DEFAULT 0 NOT NULL
);

DROP TRIGGER IF EXISTS insert_into_marf_data;

CREATE TRIGGER insert_into_marf_data 
AFTER INSERT ON marf_data 
FOR EACH ROW 
BEGIN
    INSERT INTO 
        tmp_marf_data
        (
            is_update,
            block_id,
            block_hash,
            data,
            unconfirmed,
            external_offset,
            external_length
        )
        VALUES
        (
            0,
            new.block_id,
            new.block_hash,
            new.data,
            new.unconfirmed,
            new.external_offset,
            new.external_length
        );
END;

DROP TRIGGER IF EXISTS update_marf_data;

CREATE TRIGGER update_marf_data 
AFTER UPDATE ON marf_data 
FOR EACH ROW 
BEGIN
    INSERT INTO 
        tmp_marf_data
        (
            is_update,
            block_id,
            block_hash,
            data,
            unconfirmed,
            external_offset,
            external_length
        )
        VALUES
        (
            1,
            new.block_id,
            new.block_hash,
            new.data,
            new.unconfirmed,
            new.external_offset,
            new.external_length
        );
END;
"#;

pub const INDEX_DB_INSTRUMENTATION_SQL: &str = r#"
"#;