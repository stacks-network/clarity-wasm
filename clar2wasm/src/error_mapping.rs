use clarity::vm::errors::{Error, RuntimeErrorType, ShortReturnType};
use clarity::vm::types::ResponseData;
use clarity::vm::Value;

const LOG2_ERROR_MESSAGE: &str = "log2 must be passed a positive integer";
const SQRTI_ERROR_MESSAGE: &str = "sqrti must be passed a positive integer";
const POW_ERROR_MESSAGE: &str = "Power argument to (pow ...) must be a u32 integer";

pub(crate) fn runtime_map(error_code: i32) -> Error {
    match error_code {
        0 => Error::Runtime(RuntimeErrorType::ArithmeticOverflow, Some(Vec::new())),
        1 => Error::Runtime(RuntimeErrorType::ArithmeticUnderflow, Some(Vec::new())),
        2 => Error::Runtime(RuntimeErrorType::DivisionByZero, Some(Vec::new())),
        3 => Error::Runtime(
            RuntimeErrorType::Arithmetic(LOG2_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        4 => Error::Runtime(
            RuntimeErrorType::Arithmetic(SQRTI_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        5 => Error::Runtime(RuntimeErrorType::UnwrapFailure, Some(Vec::new())),
        7 => Error::ShortReturn(ShortReturnType::AssertionFailed(Value::Response(
            ResponseData {
                committed: false,
                data: Box::new(Value::UInt(42)),
            },
        ))),
        8 => Error::Runtime(
            RuntimeErrorType::Arithmetic(POW_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        _ => panic!("Runtime error code {} not supported", error_code),
    }
}
