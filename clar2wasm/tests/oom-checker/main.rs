pub mod unit_tests;

use clar2wasm::compile;
use clar2wasm::datastore::Datastore;
use clar2wasm::tools::{crosscheck, TestConfig};
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::errors::{CheckErrors, Error};
use clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::Value;

pub fn as_oom_check_snippet(snippet: &str) -> String {
    let version = TestConfig::clarity_version();
    let epoch = TestConfig::latest_epoch();

    let compiled_module = Datastore::new()
        .as_analysis_db()
        .execute(|analysis_db| {
            compile(
                snippet,
                &QualifiedContractIdentifier::new(
                    StandardPrincipalData::transient(),
                    ("foo").into(),
                ),
                LimitedCostTracker::new_free(),
                version,
                epoch,
                analysis_db,
            )
            .map_err(|e| CheckErrors::Expects(format!("Compilation failure {e:?}")))
        })
        .expect("Could not compile snippet")
        .module;

    let memory_pages = compiled_module
        .memories
        .iter()
        .next()
        .expect("Couldn't find a memory")
        .initial as usize;
    let stack_pointer_value = match compiled_module
        .globals
        .iter()
        .find(|g| g.name.as_ref().is_some_and(|name| name == "stack-pointer"))
        .expect("Couldn't find stack-pointer global")
        .kind
    {
        walrus::GlobalKind::Local(walrus::InitExpr::Value(walrus::ir::Value::I32(val))) => {
            val as usize
        }
        _ => unreachable!("stack-pointer should be a locally declared global with a i32 value"),
    };

    let free_space_on_memory_page = memory_pages * 65536 - stack_pointer_value;

    dbg!(format!(
        "(define-constant ignore 0x{})\n{snippet}",
        "00".repeat({
            // we will need 8 bytes for the (offset, size) and 6 bytes for the name "ignore"
            free_space_on_memory_page
                .checked_sub(14)
                // we add a page if we don't have 14 remaining bytes
                .unwrap_or(free_space_on_memory_page + 65536 - 8 - 6)
        })
    ))
}

pub fn crosscheck_oom(snippet: &str, expected: Result<Option<Value>, Error>) {
    crosscheck(&as_oom_check_snippet(snippet), expected);
}
