use diesel::prelude::*;

table! {
    data_table (key) {
        key -> Text,
        value -> Text
    }
}

table! {
    metadata_table (key, blockhash) {
        key -> Text,
        blockhash -> Text,
        value -> Text
    }
}
