use walrus::InstrSeqBuilder;

use crate::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use lazy_static::lazy_static;
use std::collections::HashMap;

pub mod arithmetic;
pub mod comparison;
pub mod data_vars;
pub mod list_manipulation;
pub mod traits;
pub mod tuples;

pub(crate) static WORDS: &[&'static dyn Word] = &[
    &arithmetic::Add,
    &arithmetic::Sub,
    &arithmetic::Mul,
    &arithmetic::Div,
    &arithmetic::Sqrti,
    &arithmetic::Power,
    &tuples::TupleCons,
    &tuples::TupleGet,
    &tuples::TupleMerge,
    &comparison::CmpLess,
    &comparison::CmpGreater,
    &comparison::CmpLeq,
    &comparison::CmpGeq,
    &list_manipulation::Concat,
    &list_manipulation::ListCons,
    &data_vars::DefineDataVar,
    &data_vars::SetDataVar,
];

pub trait Word: Sync + core::fmt::Debug {
    fn name(&self) -> ClarityName;

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError>;
}

lazy_static! {
    static ref WORDS_BY_NAME: HashMap<ClarityName, &'static dyn Word> = {
        let mut wbn = HashMap::new();

        for word in WORDS {
            wbn.insert(word.name(), &**word);
        }

        wbn
    };
}

pub fn lookup(name: &str) -> Option<&'static dyn Word> {
    WORDS_BY_NAME.get(name).copied()
}
