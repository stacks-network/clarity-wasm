use clarity::types::StacksEpochId;
use clarity::vm::errors::{CheckErrors, Error, RuntimeErrorType, ShortReturnType, WasmError};
use clarity::vm::ClarityVersion;
use wasmtime::{AsContextMut, Instance, Trap};

use crate::wasm_utils::{
    read_from_wasm_indirect, read_identifier_from_wasm, signature_from_string,
};

const LOG2_ERROR_MESSAGE: &str = "log2 must be passed a positive integer";
const SQRTI_ERROR_MESSAGE: &str = "sqrti must be passed a positive integer";
const POW_ERROR_MESSAGE: &str = "Power argument to (pow ...) must be a u32 integer";

/// Represents various error conditions that can occur
/// during Clarity contract execution
/// or other Stacks blockchain operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorMap {
    /// Indicates that the error is not related to Clarity contract execution.
    NotClarityError = -1,

    /// Represents an arithmetic overflow error in Clarity contract execution.
    /// This occurs when a calculation exceeds the maximum value representable.
    ArithmeticOverflow = 0,

    /// Represents an arithmetic underflow error in Clarity contract execution.
    /// This occurs when a calculation results in a value below the minimum representable value.
    ArithmeticUnderflow = 1,

    /// Indicates an attempt to divide by zero in a Clarity contract.
    DivisionByZero = 2,

    /// Represents an error in calculating the logarithm base 2 in a Clarity contract.
    /// This could occur for negative inputs.
    ArithmeticLog2Error = 3,

    /// Represents an error in calculating the integer square root in a Clarity contract.
    /// This could occur for negative inputs.
    ArithmeticSqrtiError = 4,

    /// Indicates an error in constructing a type, possibly due to invalid parameters.
    BadTypeConstruction = 5,

    /// Represents a deliberate panic in contract execution,
    /// usually triggered by `(unwrap-panic...)` and `(unwrap-err-panic...)`.
    Panic = 6,

    /// Indicates a failure in an assertion that was expected to cause a short return,
    /// usually triggered by `(asserts!...)`.
    ShortReturnAssertionFailure = 7,

    /// Represents an error in exponentiation operations in a Clarity contract.
    /// This could occur for invalid bases or exponents.
    ArithmeticPowError = 8,

    /// Indicates an attempt to use a name that is already in use, possibly for a variable or function.
    NameAlreadyUsed = 9,

    /// A catch-all for errors that are not mapped to specific error codes.
    /// This might be used for unexpected or unclassified errors.
    NotMapped = 99,
}

impl From<i32> for ErrorMap {
    fn from(error_code: i32) -> Self {
        match error_code {
            -1 => ErrorMap::NotClarityError,
            0 => ErrorMap::ArithmeticOverflow,
            1 => ErrorMap::ArithmeticUnderflow,
            2 => ErrorMap::DivisionByZero,
            3 => ErrorMap::ArithmeticLog2Error,
            4 => ErrorMap::ArithmeticSqrtiError,
            5 => ErrorMap::BadTypeConstruction,
            6 => ErrorMap::Panic,
            7 => ErrorMap::ShortReturnAssertionFailure,
            8 => ErrorMap::ArithmeticPowError,
            9 => ErrorMap::NameAlreadyUsed,
            _ => ErrorMap::NotMapped,
        }
    }
}

pub(crate) fn resolve_error(
    e: wasmtime::Error,
    instance: Instance,
    mut store: impl AsContextMut,
    epoch_id: &StacksEpochId,
    clarity_version: &ClarityVersion,
) -> Error {
    if let Some(vm_error) = e.root_cause().downcast_ref::<Error>() {
        // SAFETY:
        //
        // This unsafe operation returns the value of a location pointed by `*mut T`.
        //
        // The purpose of this code is to take the ownership of the `vm_error` value
        // since clarity::vm::errors::Error is not a Clonable type.
        //
        // Converting a `&T` (vm_error) to a `*mut T` doesn't cause any issues here
        // because the reference is not borrowed elsewhere.
        //
        // The replaced `T` value is deallocated after the operation. Therefore, the chosen `T`
        // is a dummy value, solely to satisfy the signature of the replace function
        // and not cause harm when it is deallocated.
        //
        // Specifically, Error::Wasm(WasmError::ModuleNotFound) was selected as the placeholder value.
        return unsafe {
            core::ptr::replace(
                (vm_error as *const Error) as *mut Error,
                Error::Wasm(WasmError::ModuleNotFound),
            )
        };
    }

    if let Some(vm_error) = e.root_cause().downcast_ref::<CheckErrors>() {
        // SAFETY:
        //
        // This unsafe operation returns the value of a location pointed by `*mut T`.
        //
        // The purpose of this code is to take the ownership of the `vm_error` value
        // since clarity::vm::errors::Error is not a Clonable type.
        //
        // Converting a `&T` (vm_error) to a `*mut T` doesn't cause any issues here
        // because the reference is not borrowed elsewhere.
        //
        // The replaced `T` value is deallocated after the operation. Therefore, the chosen `T`
        // is a dummy value, solely to satisfy the signature of the replace function
        // and not cause harm when it is deallocated.
        //
        // Specifically, CheckErrors::ExpectedName was selected as the placeholder value.
        return unsafe {
            let err = core::ptr::replace(
                (vm_error as *const CheckErrors) as *mut CheckErrors,
                CheckErrors::ExpectedName,
            );

            <CheckErrors as std::convert::Into<Error>>::into(err)
        };
    }

    // Check if the error is caused by
    // an unreachable Wasm trap.
    //
    // In this case, runtime errors are handled
    // by being mapped to the corresponding ClarityWasm Errors.
    if let Some(Trap::UnreachableCodeReached) = e.root_cause().downcast_ref::<Trap>() {
        return from_runtime_error_code(instance, &mut store, e, epoch_id, clarity_version);
    }

    // All other errors are treated as general runtime errors.
    Error::Wasm(WasmError::Runtime(e))
}

fn from_runtime_error_code(
    instance: Instance,
    mut store: impl AsContextMut,
    e: wasmtime::Error,
    epoch_id: &StacksEpochId,
    clarity_version: &ClarityVersion,
) -> Error {
    let global = "runtime-error-code";
    let runtime_error_code = instance
        .get_global(&mut store, global)
        .and_then(|glob| glob.get(&mut store).i32())
        .unwrap_or_else(|| panic!("Could not find {global} global with i32 value"));

    match ErrorMap::from(runtime_error_code) {
        ErrorMap::NotClarityError => Error::Wasm(WasmError::Runtime(e)),
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
        ErrorMap::BadTypeConstruction => {
            Error::Runtime(RuntimeErrorType::BadTypeConstruction, Some(Vec::new()))
        }
        ErrorMap::Panic => {
            // TODO: see issue: #531
            // This RuntimeErrorType::UnwrapFailure need to have a proper context.
            Error::Runtime(RuntimeErrorType::UnwrapFailure, Some(Vec::new()))
        }
        ErrorMap::ShortReturnAssertionFailure => {
            let val_offset = instance
                .get_global(&mut store, "runtime-error-value-offset")
                .and_then(|glob| glob.get(&mut store).i32())
                .unwrap_or_else(|| {
                    panic!("Could not find $runtime-error-value-offset global with i32 value")
                });

            let type_ser_offset = instance
                .get_global(&mut store, "runtime-error-type-ser-offset")
                .and_then(|glob| glob.get(&mut store).i32())
                .unwrap_or_else(|| {
                    panic!("Could not find $runtime-error-type-ser-offset global with i32 value")
                });

            let type_ser_len = instance
                .get_global(&mut store, "runtime-error-type-ser-len")
                .and_then(|glob| glob.get(&mut store).i32())
                .unwrap_or_else(|| {
                    panic!("Could not find $runtime-error-type-ser-len global with i32 value")
                });

            let memory = instance
                .get_memory(&mut store, "memory")
                .unwrap_or_else(|| panic!("Could not find wasm instance memory"));

            let type_ser_str =
                read_identifier_from_wasm(memory, &mut store, type_ser_offset, type_ser_len)
                    .unwrap_or_else(|e| panic!("Could not recover stringified type: {e}"));

            let value_ty = signature_from_string(&type_ser_str, *clarity_version, *epoch_id)
                .unwrap_or_else(|e| panic!("Could not recover thrown value: {e}"));

            let clarity_val =
                read_from_wasm_indirect(memory, &mut store, &value_ty, val_offset, *epoch_id)
                    .unwrap_or_else(|e| panic!("Could not read thrown value from memory: {e}"));

            Error::ShortReturn(ShortReturnType::AssertionFailed(clarity_val))
        }
        ErrorMap::ArithmeticPowError => Error::Runtime(
            RuntimeErrorType::Arithmetic(POW_ERROR_MESSAGE.into()),
            Some(Vec::new()),
        ),
        ErrorMap::NameAlreadyUsed => {
            let runtime_error_arg_offset = instance
                .get_global(&mut store, "runtime-error-arg-offset")
                .and_then(|glob| glob.get(&mut store).i32())
                .unwrap_or_else(|| {
                    panic!("Could not find $runtime-error-arg-offset global with i32 value")
                });

            let runtime_error_arg_len = instance
                .get_global(&mut store, "runtime-error-arg-len")
                .and_then(|glob| glob.get(&mut store).i32())
                .unwrap_or_else(|| {
                    panic!("Could not find $runtime-error-arg-len global with i32 value")
                });

            let memory = instance
                .get_memory(&mut store, "memory")
                .unwrap_or_else(|| panic!("Could not find wasm instance memory"));
            let arg_name = read_identifier_from_wasm(
                memory,
                &mut store,
                runtime_error_arg_offset,
                runtime_error_arg_len,
            )
            .unwrap_or_else(|e| panic!("Could not recover arg_name: {e}"));

            Error::Unchecked(CheckErrors::NameAlreadyUsed(arg_name))
        }
        _ => panic!("Runtime error code {} not supported", runtime_error_code),
    }
}
