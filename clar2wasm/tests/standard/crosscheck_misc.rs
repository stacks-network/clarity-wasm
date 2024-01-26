use clar2wasm::tools::{crosscheck, evaluate};

#[test]
fn is_in_mainnet() {
    crosscheck(
        "
(define-public (mainnet)
  (ok is-in-mainnet))

(mainnet)
",
        evaluate("(ok false)"),
    );
}

#[test]
fn is_in_regtest() {
    crosscheck(
        "
(define-public (regtest)
  (ok is-in-regtest))

(regtest)
",
        evaluate("(ok false)"),
    );
}
