use clar2wasm::tools::crosscheck;
use clarity::vm::types::TupleData;
use clarity::vm::Value;
use proptest::prelude::*;

use crate::PropValue;

proptest! {
    #[test]
    fn subsequent_func_calls_dont_erase_previous_results(
        result1 in PropValue::any(),
        result2 in PropValue::any(),
    ) {
        let snippet = format!(
            r#"
                (define-private (foo) {result1})
                (define-private (bar) {result2})

                {{ foo: (foo), bar: (bar) }}
            "#
        );

        let expected = Value::from(
            TupleData::from_data(vec![
                ("foo".into(), result1.into()),
                ("bar".into(), result2.into()),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }
}
