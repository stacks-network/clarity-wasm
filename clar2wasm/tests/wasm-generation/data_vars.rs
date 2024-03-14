use clar2wasm::tools::crosscheck;
use clarity::vm::types::ResponseData;
use clarity::vm::Value::Response;
use proptest::proptest;

use crate::{PropValue, TypePrinter};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_var_get(
        val in PropValue::any()
    ) {
        let snippet = format!(r#"
            (define-data-var var-name {} {})
            (define-private (foo) (ok (var-get var-name)))
            (foo)
        "#, val.type_string(), val);

        crosscheck(
            &snippet,
            Ok(Some(
                Response(
                    ResponseData {
                        committed: true,
                        data: Box::new(val.into())
                    }
                )
            ))
        )
    }
}
