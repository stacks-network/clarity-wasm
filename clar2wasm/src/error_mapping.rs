use clarity::vm::errors::{Error, RuntimeErrorType, ShortReturnType};
use clarity::vm::types::ResponseData;
use clarity::vm::Value;
use wasmtime::{AsContextMut, Instance};

const LOG2_ERROR_MESSAGE: &str = "log2 must be passed a positive integer";
const SQRTI_ERROR_MESSAGE: &str = "sqrti must be passed a positive integer";
const POW_ERROR_MESSAGE: &str = "Power argument to (pow ...) must be a u32 integer";

pub enum ErrorMap {
    ArithmeticOverflow = 0,
    ArithmeticUnderflow = 1,
    DivisionByZero = 2,
    ArithmeticLog2Error = 3,
    ArithmeticSqrtiError = 4,
    UnwrapFailure = 5,
    Panic = 6,
    ShortReturnAssertionFailure = 7,
    ArithmeticPowError = 8,
    NotMapped = 99,
}

impl ErrorMap {
    pub fn from(error_code: i32) -> ErrorMap {
        match error_code {
            0 => ErrorMap::ArithmeticOverflow,
            1 => ErrorMap::ArithmeticUnderflow,
            2 => ErrorMap::DivisionByZero,
            3 => ErrorMap::ArithmeticLog2Error,
            4 => ErrorMap::ArithmeticSqrtiError,
            5 => ErrorMap::UnwrapFailure,
            6 => ErrorMap::Panic,
            7 => ErrorMap::ShortReturnAssertionFailure,
            8 => ErrorMap::ArithmeticPowError,
            _ => ErrorMap::NotMapped,
        }
    }
}

pub(crate) fn map(instance: Instance, mut store: impl AsContextMut) -> Error {
    let global = "runtime_error_code";
    let runtime_error_code = instance
        .get_global(&mut store, global)
        .and_then(|glob| glob.get(&mut store).i32())
        // TODO: change that to a proper error when PR below is merged on stacks-core.
        // https://github.com/stacks-network/stacks-core/pull/4878 introduces a
        // generic error handling for global variables.
        .unwrap_or_else(|| panic!("Could not find {global} global with i32 value"));

    match ErrorMap::from(runtime_error_code) {
        ErrorMap::ArithmeticOverflow => {
            Error::Runtime(RuntimeErrorType::ArithmeticOverflow, Some(Vec::new()))
        }
        ErrorMap::ArithmeticUnderflow => {
            Error::Runtime(RuntimeErrorType::ArithmeticUnderflow, Some(Vec::new()))
        }
        ErrorMap::DivisionByZero => {
            Error::Runtime(RuntimeErrorType::DivisionByZero, Some(Vec::new()))
        }
        ErrorMap::ArithmeticLog2Error => Error::Runtime(
            RuntimeErrorType::Arithmetic(LOG2_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        ErrorMap::ArithmeticSqrtiError => Error::Runtime(
            RuntimeErrorType::Arithmetic(SQRTI_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        ErrorMap::UnwrapFailure => {
            Error::Runtime(RuntimeErrorType::UnwrapFailure, Some(Vec::new()))
        }
        ErrorMap::Panic => {
            panic!("An error has been detected in the code")
        }
        ErrorMap::ShortReturnAssertionFailure => Error::ShortReturn(
            ShortReturnType::AssertionFailed(Value::Response(ResponseData {
                committed: false,
                data: Box::new(Value::UInt(42)),
            })),
        ),
        ErrorMap::ArithmeticPowError => Error::Runtime(
            RuntimeErrorType::Arithmetic(POW_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        _ => panic!("Runtime error code {} not supported", runtime_error_code),
    }
}
