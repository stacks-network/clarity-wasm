use diesel::prelude::*;

table! {
    epochs (start_block_height, epoch_id) {
        start_block_height -> Integer,
        end_block_height -> Integer,
        epoch_id -> Integer,
        block_limit -> Text,
        network_epoch -> Integer
    }
}

table! {
    block_commits (txid, sortition_id) {
        txid -> Text,
        vtxindex -> Integer,
        block_height -> Integer,
        burn_header_hash -> Text,
        sortition_id -> Text,
        block_header_hash -> Text,
        new_seed -> Text,
        parent_block_ptr -> Integer,
        parent_vtxindex -> Integer,
        key_block_ptr -> Integer,
        key_vtxindex -> Integer,
        memo -> Text,
        commit_outs -> Text,
        burn_fee -> Text,
        sunset_burn -> Text,
        input -> Text,
        apparent_sender -> Text,
        burn_parent_modulus -> Integer,
    }
}

table! {
    snapshots (sortition_id) {
        block_height -> Integer,
        burn_header_hash -> Text,
        sortition_id -> Text,
        parent_sortition_id -> Text,
        burn_header_timestamp -> BigInt,
        parent_burn_header_hash -> Text,
        consensus_hash -> Text,
        ops_hash -> Text,
        total_burn -> Text,
        sortition -> Integer,
        sortition_hash -> Text,
        winning_block_txid -> Text,
        winning_stacks_block_hash -> Text,
        index_root -> Text,
        num_sortitions -> Integer,
        stacks_block_accepted -> Integer,
        stacks_block_height -> Integer,
        arrival_index -> Integer,
        canonical_stacks_tip_height -> Integer,
        canonical_stacks_tip_hash -> Text,
        canonical_stacks_tip_consensus_hash -> Text,
        pox_valid -> Integer,
        accumulated_coinbase_ustx -> Text,
        pox_payouts -> Text,
    }
}

table! {
    ast_rule_heights (ast_rule_id) {
        ast_rule_id -> Integer,
        block_height -> Integer
    }
}
