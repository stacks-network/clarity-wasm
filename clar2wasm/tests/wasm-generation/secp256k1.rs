use clar2wasm::tools::crosscheck_compare_only;
use proptest::proptest;

use crate::buffer;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_secp256k1_recover_generic(msg in buffer(32), sig in buffer(65))
    {
        crosscheck_compare_only(
            &format!("(secp256k1-recover? {msg} {sig})")
        )
    }

    #[test]
    fn crossprop_secp256k1_recover_recid(msg in buffer(32), sig in buffer(64))
    {
        for recid in 0..4 {
            crosscheck_compare_only(
                &format!("(secp256k1-recover? {msg} {sig}{recid:02X})")
            )
        }
    }

    #[test]
    fn crossprop_secp256k1_recover_recid_23(msg in buffer(32), sig in buffer(48))
    {
        // Generate "low" R signatures to hope for valid recovery_id=(2|3) values
        for recid in 0..4 {
            crosscheck_compare_only(
                &format!("(secp256k1-recover? {msg} 0x00000000000000000000000000000000{}{recid:02X})", &sig.to_string()[2..])
            )
        }
    }
}
