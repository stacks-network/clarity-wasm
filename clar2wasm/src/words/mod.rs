use std::collections::HashMap;

use clarity::vm::{ClarityName, SymbolicExpression};
use lazy_static::lazy_static;
use walrus::InstrSeqBuilder;

use crate::{GeneratorError, WasmGenerator};

pub mod arithmetic;
pub mod bindings;
pub mod bitwise;
pub mod blockinfo;
pub mod buff_to_integer;
pub mod comparison;
pub mod conditionals;
pub mod consensus_buff;
pub mod constants;
pub mod contract;
pub mod control_flow;
pub mod conversion;
pub mod data_vars;
pub mod default_to;
pub mod enums;
pub mod equal;
pub mod functions;
pub mod hashing;
pub mod logical;
pub mod maps;
pub mod noop;
pub mod options;
pub mod principal;
pub mod print;
pub mod responses;
pub mod secp256k1;
pub mod sequences;
pub mod stx;
pub mod tokens;
pub mod traits;
pub mod tuples;

pub(crate) static WORDS: &[&'static dyn Word] = &[
    &arithmetic::Add,
    &arithmetic::Div,
    &arithmetic::Mul,
    &arithmetic::Power,
    &arithmetic::Sqrti,
    &arithmetic::Sub,
    &bindings::Let,
    &bitwise::BitwiseAnd,
    &bitwise::BitwiseLShift,
    &bitwise::BitwiseNot,
    &bitwise::BitwiseOr,
    &bitwise::BitwiseRShift,
    &bitwise::BitwiseXor,
    &bitwise::Xor,
    &blockinfo::AtBlock,
    &blockinfo::GetBlockInfo,
    &blockinfo::GetBurnBlockInfo,
    &buff_to_integer::BuffToIntBe,
    &buff_to_integer::BuffToIntLe,
    &buff_to_integer::BuffToUintBe,
    &buff_to_integer::BuffToUintLe,
    &comparison::CmpGeq,
    &comparison::CmpGreater,
    &comparison::CmpLeq,
    &comparison::CmpLess,
    &conditionals::And,
    &conditionals::Asserts,
    &conditionals::Filter,
    &conditionals::If,
    &conditionals::Match,
    &conditionals::Or,
    &conditionals::Try,
    &conditionals::Unwrap,
    &conditionals::UnwrapErr,
    &consensus_buff::To,
    &consensus_buff::From,
    &constants::DefineConstant,
    &contract::AsContract,
    &contract::ContractCall,
    &control_flow::Begin,
    &control_flow::UnwrapErrPanic,
    &control_flow::UnwrapPanic,
    &conversion::IntToAscii,
    &conversion::IntToUtf8,
    &conversion::StringToInt,
    &conversion::StringToUint,
    &data_vars::DefineDataVar,
    &data_vars::GetDataVar,
    &data_vars::SetDataVar,
    &default_to::DefaultTo,
    &enums::ClarityErr,
    &enums::ClarityOk,
    &enums::ClaritySome,
    &equal::IsEq,
    &equal::IndexOf::Alias,
    &equal::IndexOf::Original,
    &functions::DefinePrivateFunction,
    &functions::DefinePublicFunction,
    &functions::DefineReadonlyFunction,
    &hashing::Hash160,
    &hashing::Keccak256,
    &hashing::Sha256,
    &hashing::Sha512,
    &hashing::Sha512_256,
    &logical::Not,
    &maps::MapDefinition,
    &maps::MapDelete,
    &maps::MapGet,
    &maps::MapInsert,
    &maps::MapSet,
    &noop::ContractOf,
    &noop::ToInt,
    &noop::ToUint,
    &options::IsNone,
    &options::IsSome,
    &principal::Construct,
    &principal::Destruct,
    &principal::IsStandard,
    &principal::PrincipalOf,
    &print::Print,
    &responses::IsErr,
    &responses::IsOk,
    &secp256k1::Recover,
    &secp256k1::Verify,
    &sequences::Append,
    &sequences::AsMaxLen,
    &sequences::Concat,
    &sequences::ElementAt::Alias,
    &sequences::ElementAt::Original,
    &sequences::Fold,
    &sequences::Len,
    &sequences::ListCons,
    &sequences::ReplaceAt,
    &sequences::Slice,
    &stx::StxBurn,
    &stx::StxGetAccount,
    &stx::StxGetBalance,
    &stx::StxTransfer,
    &stx::StxTransferMemo,
    &tokens::BurnFungibleToken,
    &tokens::BurnNonFungibleToken,
    &tokens::DefineFungibleToken,
    &tokens::DefineNonFungibleToken,
    &tokens::GetBalanceOfFungibleToken,
    &tokens::GetOwnerOfNonFungibleToken,
    &tokens::GetSupplyOfFungibleToken,
    &tokens::MintFungibleToken,
    &tokens::MintNonFungibleToken,
    &tokens::TransferFungibleToken,
    &tokens::TransferNonFungibleToken,
    &traits::DefineTrait,
    &traits::ImplTrait,
    &traits::UseTrait,
    &tuples::TupleCons,
    &tuples::TupleGet,
    &tuples::TupleMerge,
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

#[cfg(test)]
mod tests {
    use clarity::vm::functions::define::DefineFunctions;
    use clarity::vm::functions::NativeFunctions;
    use clarity::vm::variables::NativeVariables;

    #[test]
    fn check_for_duplicates() {
        use std::collections::HashSet;

        let mut names = HashSet::new();

        for word in super::WORDS {
            assert!(
                names.insert(word.name()),
                "duplicate word: {:?}",
                word.name()
            );
        }
    }

    #[test]
    fn check_for_non_reserved_words() {
        for word in super::WORDS {
            // Printing each word also gets us coverage on the Debug impl.
            println!("{:?} => {}", word, word.name());
            assert!(
                DefineFunctions::lookup_by_name(&word.name()).is_some()
                    || NativeFunctions::lookup_by_name(&word.name()).is_some()
                    || NativeVariables::lookup_by_name(&word.name()).is_some(),
            );
        }
    }
}
