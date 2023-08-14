mod util;

#[cfg(test)]
mod tests {
    use wasmtime::Val;

    use crate::util::{ClarityWasmResult, WasmtimeHelper};

    #[test]
    fn add() {
        let mut helper = WasmtimeHelper::new("add");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("simple", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 3);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn call_private_with_args_nested() {
        let mut helper = WasmtimeHelper::new("call-private-with-args");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("call-it", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 3);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn call_public() {
        let mut helper = WasmtimeHelper::new("call-public");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("simple", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 42);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn call_public_nested() {
        let mut helper = WasmtimeHelper::new("call-public");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("call-it", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 42);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn call_public_with_args() {
        let mut helper = WasmtimeHelper::new("call-public-with-args");

        let params = &[Val::I64(0), Val::I64(20), Val::I64(0), Val::I64(22)];

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("simple", params)
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 42);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn call_public_with_args_nested() {
        let mut helper = WasmtimeHelper::new("call-public-with-args");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("call-it", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 3);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn define_public_err() {
        let mut helper = WasmtimeHelper::new("define-public-err");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("simple", &[])
        {
            assert_eq!(indicator, 0);
            assert!(ok_value.is_none());
            assert!(err_value.is_some());
            if let ClarityWasmResult::Int { high, low } = *err_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 42);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }

    #[test]
    fn define_public_ok() {
        let mut helper = WasmtimeHelper::new("define-public-ok");

        if let ClarityWasmResult::Response {
            indicator,
            ok_value,
            err_value,
        } = helper.call_public_function("simple", &[])
        {
            assert_eq!(indicator, 1);
            assert!(ok_value.is_some());
            assert!(err_value.is_none());
            if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                assert_eq!(high, 0);
                assert_eq!(low, 42);
            }
        } else {
            panic!("Unexpected result received from WASM function call.");
        }
    }
}
