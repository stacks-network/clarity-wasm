use crate::stacks;

pub fn mainnet_boot_data() -> stacks::ChainStateBootData {
    stacks::ChainStateBootData {
        initial_balances: vec![],
        post_flight_callback: None,
        first_burnchain_block_hash: stacks::BurnchainHeaderHash::from_hex(
            stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH,
        )
        .unwrap(),
        first_burnchain_block_height: stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT as u32,
        first_burnchain_block_timestamp: stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP,
        pox_constants: stacks::PoxConstants::mainnet_default(),
        get_bulk_initial_lockups: Some(Box::new(|| {
            Box::new(stacks::GenesisData::new(false).read_lockups().map(|item| {
                stacks::ChainstateAccountLockup {
                    address: item.address,
                    amount: item.amount,
                    block_height: item.block_height,
                }
            }))
        })),
        get_bulk_initial_balances: Some(Box::new(|| {
            Box::new(stacks::GenesisData::new(false).read_balances().map(|item| {
                stacks::ChainstateAccountBalance {
                    address: item.address,
                    amount: item.amount,
                }
            }))
        })),
        get_bulk_initial_namespaces: Some(Box::new(|| {
            Box::new(
                stacks::GenesisData::new(false)
                    .read_namespaces()
                    .map(|item| stacks::ChainstateBNSNamespace {
                        namespace_id: item.namespace_id,
                        importer: item.importer,
                        buckets: item.buckets,
                        base: item.base as u64,
                        coeff: item.coeff as u64,
                        nonalpha_discount: item.nonalpha_discount as u64,
                        no_vowel_discount: item.no_vowel_discount as u64,
                        lifetime: item.lifetime as u64,
                    }),
            )
        })),
        get_bulk_initial_names: Some(Box::new(|| {
            Box::new(stacks::GenesisData::new(false).read_names().map(|item| {
                stacks::ChainstateBNSName {
                    fully_qualified_name: item.fully_qualified_name,
                    owner: item.owner,
                    zonefile_hash: item.zonefile_hash,
                }
            }))
        })),
    }
}
