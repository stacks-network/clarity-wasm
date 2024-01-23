use clar2wasm::tools::crosscheck;
use proptest::proptest;

use crate::PropValue;

proptest! {
  #[test]
  fn crossprop_let(v1 in PropValue::any()) {
    crosscheck(
        &format!("(let ((x {v1})) x)"),
        Ok(Some(v1.into()))
    )
  }
}
