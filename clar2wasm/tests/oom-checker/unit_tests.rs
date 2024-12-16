use clarity::vm::types::PrincipalData;
use clarity::vm::Value;

use crate::crosscheck_oom;

#[test]
#[ignore = "issue #585"]
fn principal_of_oom() {
    crosscheck_oom(
        "(principal-of? 0x03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110)",
        Ok(Some(
            Value::okay(
                PrincipalData::parse("ST1AW6EKPGT61SQ9FNVDS17RKNWT8ZP582VF9HSCP")
                    .unwrap()
                    .into(),
            )
            .unwrap(),
        )),
    )
}
