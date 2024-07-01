#[cfg(test)]
pub mod bin_tests;
#[cfg(test)]
pub mod lib_tests;

#[cfg(test)]
mod tests {
    #[test]
    /// Specific test for the fix of [issue 340](https://github.com/stacks-network/clarity-wasm/issues/340)
    fn test_bns_contract_in_epoch2_4() {
        let bns = std::fs::read_to_string(
            env!("CARGO_MANIFEST_DIR").to_owned() + "/tests/contracts/boot-contracts/bns.clar",
        )
        .expect("Can't find bns contract");

        assert!(clar2wasm::tools::evaluate_at(
            &bns,
            clarity::types::StacksEpochId::Epoch20,
            clarity::vm::version::ClarityVersion::Clarity1,
        )
        .is_ok());

        assert!(clar2wasm::tools::evaluate_at(
            &bns,
            clarity::types::StacksEpochId::Epoch24,
            clarity::vm::version::ClarityVersion::Clarity2,
        )
        .is_ok());
    }
}
