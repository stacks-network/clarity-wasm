use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm_tests::WasmtimeHelper;
use clarity::vm::ast::ContractAST;
use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        contexts::GlobalContext,
        costs::LimitedCostTracker,
        database::ClarityDatabase,
        types::{QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion, ContractContext, ContractName,
    },
};
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, Session, SessionSettings,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn wasm_fold_add_square(c: &mut Criterion) {
    c.bench_function("wasm_fold_add_square", |b| {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("fold-bench"),
        );
        let mut datastore = Datastore::new();
        let constants = StacksConstants {
            burn_start_height: 0,
            pox_prepare_length: 0,
            pox_reward_cycle_length: 0,
            pox_rejection_fraction: 0,
            epoch_21_start_height: 0,
        };
        let burn_datastore = BurnDatastore::new(constants);
        let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        conn.begin();
        conn.set_clarity_epoch_version(StacksEpochId::Epoch24);
        conn.commit();
        let cost_tracker = LimitedCostTracker::new_free();
        let mut global_context = GlobalContext::new(
            false,
            CHAIN_ID_TESTNET,
            conn,
            cost_tracker,
            StacksEpochId::Epoch24,
        );
        let mut contract_context =
            ContractContext::new(contract_id.clone(), ClarityVersion::Clarity2);

        global_context.begin();
        {
            let mut helper = WasmtimeHelper::new_from_file(
                contract_id,
                &mut global_context,
                &mut contract_context,
            );

            b.iter(|| {
                helper.call_public_function("fold-add-square", &[]);
            });
        }
        global_context.commit().unwrap();
    });
}

fn interp_fold_add_square(c: &mut Criterion) {
    // Setup the session with the Clarity contract first
    let mut session = Session::new(SessionSettings::default());
    let contract_source = include_str!("../contracts/fold-bench.clar");

    let contract = ClarityContract {
        name: "fold-bench".to_string(),
        code_source: ClarityCodeSource::ContractInMemory(contract_source.to_string()),
        clarity_version: ClarityVersion::Clarity2,
        epoch: StacksEpochId::latest(),
        deployer: ContractDeployer::Address(
            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM".to_string(),
        ),
    };

    let mut ast: Option<ContractAST> = None;
    session
        .deploy_contract(&contract, None, false, None, &mut ast)
        .unwrap();
    session
        .eval(
            "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.fold-bench fold-add-square)"
                .to_string(),
            None,
            false,
        )
        .unwrap();

    c.bench_function("interp_fold_add_square", |b| {
        b.iter(|| {
            session
                .eval(
                    "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.fold-bench fold-add-square)"
                        .to_string(),
                    None,
                    false,
                )
                .unwrap();
        })
    });
}

criterion_group!(
    fold_add_square,
    wasm_fold_add_square,
    interp_fold_add_square
);
criterion_main!(fold_add_square);
