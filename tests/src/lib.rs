#[cfg(test)]
pub mod bin_tests;
#[cfg(test)]
pub mod lib_tests;

#[cfg(test)]
mod tests {
    #[test]
    /// Specific test for the fix of [issue 340](https://github.com/stacks-network/clarity-wasm/issues/340)
    fn test_bns_contract_latest() {
        let bns =
            std::fs::read_to_string(env!("CARGO_MANIFEST_DIR").to_owned() + "/contracts/bns.clar")
                .expect("Can't find bns contract");

        // check for normal behavior with latest epoch/version
        clar2wasm::tools::crosscheck(&bns, Ok(None));

        // check with the issue's problematic epoch
        assert!(clar2wasm::tools::evaluate_at(
            &bns,
            clarity::types::StacksEpochId::Epoch20,
            clarity::vm::version::ClarityVersion::latest(),
        )
        .is_ok());
    }
}
