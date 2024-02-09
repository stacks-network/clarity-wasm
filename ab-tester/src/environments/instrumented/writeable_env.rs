use ::clarity::vm::types::ResponseData;
use color_eyre::eyre::{Result, bail};
use log::*;
use tracing::trace_span;

use crate::{
    clarity, 
    stacks, 
    environments::{
        WriteableEnv, ReadableEnv, RuntimeEnv,
        stacks_node::db::stacks_headers_db::StacksHeadersDb, 
    }, 
    context::Block
};

use super::InstrumentedEnv;

/// Implementation of [WriteableEnv] for [InstrumentedEnv].
impl WriteableEnv for InstrumentedEnv {
    fn process_block(
        &mut self, 
        block: &crate::context::Block
    ) -> Result<()>
    {
        let _ = trace_span!("process_block");

        use clarity::{
            PrincipalData, QualifiedContractIdentifier, 
            ClarityVersion, ASTRules, StandardPrincipalData, TransactionConnection
        };
        use stacks::{TransactionPayload, StacksChainState};

        let blocks_dir = self.cfg().blocks_dir().to_owned();

        if self.is_readonly() {
            bail!("[{}] environment is read-only.", self.name);
        }

        //trace!("block: {block:?}");

        // Insert this block into the app database.
        debug!("creating block in app datastore");
        self.app_db.insert_block(
            self.id,
            block.block_height()? as i32,
            block.block_hash()?.as_bytes(),
            block.index_block_hash()?,
        )?;

        match block {
            Block::Genesis(inner) => {
                info!(
                    "Reached GENESIS block: {}",
                    &inner.header.index_block_hash.to_hex()
                );
                info!("Genesis block has already been processed as a part of boot init - continuing...");
            },
            Block::Regular(inner) => {
                info!(
                    "beginning regular block: {}",
                    &inner.header.index_block_hash.to_hex()
                );

                let state = self.get_env_state_mut()?;

                let parent_consensus_hash = inner.parent_header.consensus_hash;
                let parent_block_hash = inner.parent_header.block_hash;
                let new_consensus_hash = inner.header.consensus_hash;
                let new_block_hash = inner.header.block_hash;

                debug!("parent_consensus_hash: {}, parent_block: {}, new_consensus_hash: {}, new_block: {}",
                    parent_consensus_hash.to_hex(),
                    parent_block_hash.to_hex(),
                    new_consensus_hash.to_hex(),
                    new_block_hash.to_hex()
                );

                debug!("beginning chainstate transaction/clarity tx");
                let chainstate_tx = state.chainstate.chainstate_tx_begin()?;

                debug!("beginning block");
                let mut block_conn = chainstate_tx.1.begin_block(
                    &stacks::StacksBlockId::new(&parent_consensus_hash, &parent_block_hash),
                    &stacks::StacksBlockId::new(&new_consensus_hash, &new_block_hash), 
                    &*state.headers_db, 
                    &*state.burnstate_db
                );

                for block_tx in inner.stacks_block.txs.iter() {
                    // If a sponsor has been provided, convert it to a `PrincipalData`.
                    let sponsor_addr = block_tx.sponsor_address()
                        .map(|addr| PrincipalData::Standard(StandardPrincipalData::from(addr)));
                    if sponsor_addr.is_some() {
                        debug!("a sponsor address has been provided: {:?}", &sponsor_addr);
                    }
                    
                    match &block_tx.payload {
                        TransactionPayload::SmartContract(ref contract, clarity_version) => {
                            // Use the provided Clarity version, if supplied, otherwise use latest.
                            let clarity_version = clarity_version
                                .unwrap_or(ClarityVersion::latest());
                                
                            // Construct a `QualifiedContractIdentifier` from the contract details.
                            let contract_id = QualifiedContractIdentifier::new(
                                block_tx.origin_address().into(), 
                                contract.name.clone());

                            info!("installing contract: {}", &contract_id);

                            // If a sponsor has been provided, convert it to a `PrincipalData`.
                            let sponsor_addr = block_tx.sponsor_address()
                                .map(|addr| PrincipalData::Standard(StandardPrincipalData::from(addr)));
                            if sponsor_addr.is_some() {
                                debug!("a sponsor address has been provided: {:?}", &sponsor_addr);
                            }

                            // Begin a new Clarity transaction in `target` and process the
                            // source transaction.
                            let result = block_conn.as_transaction(|tx| -> Result<()> {
                                // Perform a contract analysis so that we can get ahold of the
                                // contract's parsed AST, which is needed for the install/init
                                // phase below.
                                debug!("performing contract analysis");
                                let (contract_ast, _contract_analysis) = tx.analyze_smart_contract(
                                    &contract_id, 
                                    clarity_version, 
                                    &contract.code_body.to_string(),
                                    ASTRules::PrecheckSize
                                )?;

                                // Initialize the smart contract.
                                debug!("initializing smart contract");
                                tx.initialize_smart_contract(
                                    &contract_id, 
                                    clarity_version, 
                                    &contract_ast, 
                                    &contract.code_body.to_string(), 
                                    sponsor_addr, 
                                    |_, _| {
                                        false
                                    }).expect("failed to initialize smart contract");

                                debug!("contract initialized; committing");
                                ok!()
                            });

                            match result {
                                Ok(result) => {
                                    trace!("contract install result: {:?}", result);
                                },
                                Err(err) => {
                                    error!("contract install error: {:?}", err);
                                }
                            }
                        },
                        TransactionPayload::ContractCall(call) => {
                            info!("contract call: {}.{} by {}", call.contract_name, call.function_name, call.address);
                                
                            // Construct a `QualifiedContractIdentifier` from the contract details.
                            let contract_id = call.to_clarity_contract_id();

                            let sender_addr = PrincipalData::Standard(
                                StandardPrincipalData::from(block_tx.origin_address()));

                            // origin balance may have changed (e.g. if the origin paid the tx fee), so reload the account
                            let origin_account =
                                StacksChainState::get_account(&mut block_conn, &block_tx.origin_address().into());

                            // Begin a new Clarity transaction in `target` and replay the 
                            // contract call from `source`.
                            block_conn.as_transaction(|tx| {
                                let start = std::time::Instant::now();
                                let contract_call_result = tx.run_contract_call(
                                    &sender_addr,
                                    sponsor_addr.as_ref(), 
                                    &contract_id, 
                                    &call.function_name, 
                                    &call.function_args, 
                                    |asset_map, _| {
                                        
                                        // Check the post-conditions of the contract call, and
                                        // roll-back if they are not met.
                                        !StacksChainState::check_transaction_postconditions(
                                            &block_tx.post_conditions,
                                            &block_tx.post_condition_mode,
                                            &origin_account,
                                            asset_map,
                                        )
                                    });
                                let elapsed = start.elapsed();

                                match contract_call_result {
                                    Ok(result) => {
                                        info!("contract call success: {}ms", elapsed.as_millis());
                                        trace!("contract call result: {:?}", result);
                                    },
                                    Err(err) => {
                                        error!("contract call error: {:?}", err);
                                    }
                                }
                            });
                        },
                        stacks::TransactionPayload::Coinbase(_coinbase, _principal) => {
                            
                            warn!("coinbase");
                        },
                        stacks::TransactionPayload::TokenTransfer(address, amount , memo) => {
                            use crate::stacks::ClarityError;
                            use crate::clarity::{VmError, InterpreterError};

                            
                            let result = block_conn.as_transaction(|tx| {
                                tx.run_stx_transfer(
                                    &clarity::PrincipalData::Standard(block_tx.origin_address().into()), 
                                    address, 
                                    *amount as u128, 
                                    &clarity::BuffData { data: memo.0.to_vec() }
                                )
                            });

                            let to_principal = if let clarity::PrincipalData::Standard(principal) = address {
                                principal.to_string()
                            } else if let clarity::PrincipalData::Contract(contract) = address {
                                contract.to_string()
                            } else {
                                bail!("could not resolve to-principal")
                            };

                            match result {
                                Ok(result) => {
                                    match result.0 {
                                        ::clarity::vm::Value::Response(data) => {
                                            if data.committed {
                                                info!("token transfer: {:?} -> {:?} (STX {:?})", block_tx.origin_address().to_string(), to_principal, amount);
                                            }
                                        },
                                        _ => {
                                            warn!("token transfer: {:?} -> {:?} ({:?}): unexpected result", block_tx.origin_address(), address, amount);
                                        }
                                    }
                                },
                                Err(err) => {
                                    match err {
                                        ClarityError::Interpreter(VmError::Interpreter(InterpreterError::InsufficientBalance)) => {
                                            warn!("token transfer: {:?} -> {:?} ({:?}): insufficient balance", block_tx.origin_address(), address, amount);
                                        },
                                        _ => error!("token transfer failed: {err:?}")
                                    }
                                }
                            }

                            //warn!("token transfer");
                        },
                        stacks::TransactionPayload::PoisonMicroblock(_, _) => {
                            warn!("poison microblock");
                        }
                    }
                }

                debug!("block processed, committing");
                block_conn.seal();
                block_conn.commit_to_block(&stacks::StacksBlockId::new(&new_consensus_hash, &new_block_hash));
                //block_conn.commit_mined_block(&StacksBlockId::new(&ctx.new_consensus_hash, &ctx.new_block_hash));
                chainstate_tx.0.commit()?;

                debug!("writing block to blocks directory");
                stacks::StacksChainState::store_block(blocks_dir.to_str().expect("failed to fetch block dir"), &new_consensus_hash, &inner.stacks_block)?;
            }
        };

        self.app_db.set_environment_last_block_height(
            self.id, 
            block.block_height()? as i32)?;

        Ok(())

    }

    /// Imports chainstate from the provided source environment into this environment.
    fn import_chainstate(&self, source: &dyn ReadableEnv) -> Result<()> {
        let env_name = self.name();
        let mut headers_db = StacksHeadersDb::new(self.env_config.paths.index_db_path())?;

        // Import block headers
        debug!(
            "[{env_name}] importing block headers from '{}'...",
            source.name()
        );
        let src_block_headers_iter = source.block_headers(5000)?;
        headers_db.import_block_headers(src_block_headers_iter, Some(self.id()))?;

        // Payments
        debug!("[{env_name}] importing payments from '{}'...", source.name());
        let src_payments_iter = source.payments(5000)?;
        headers_db
            .import_payments(src_payments_iter, Some(self.id()))?;

        ok!()
    }

    /// Imports burnstate from the provided source environment into this environment.
    fn import_burnstate(&self, source: &dyn ReadableEnv) -> Result<()> {
        let env_name = self.name();

        // Snapshots
        debug!(
            "[{env_name}] importing snapshots from '{}'...",
            source.name()
        );
        let src_snapshots_iter = source.snapshots(5000)?;
        self.app_db
            .batch()
            .import_snapshots(src_snapshots_iter, Some(self.id()))?;

        // Block commits
        debug!(
            "[{env_name}] importing block commits from '{}'...",
            source.name()
        );
        let src_block_commits_iter = source.block_commits(5000)?;
        self.app_db
            .batch()
            .import_block_commits(src_block_commits_iter, Some(self.id()))?;

        // AST rules
        debug!(
            "[{env_name}] importing AST rules from '{}'...",
            source.name(),
        );
        let src_ast_rules_iter = source.ast_rules()?;
        self.app_db
            .batch()
            .import_ast_rules(src_ast_rules_iter, Some(self.id()))?;

        // Epochs
        debug!(
            "[{env_name}] importing epochs from '{}'...",
            source.name()
        );
        let src_epochs_iter = source.epochs()?;
        self.app_db
            .batch()
            .import_epochs(src_epochs_iter, Some(self.id()))?;

        ok!()
    }

    fn clear_blocks(&self) -> Result<u32> {
        let result = self.app_db
            .delete_blocks_for_environment(self.id)?;

        Ok(result)
    }
}