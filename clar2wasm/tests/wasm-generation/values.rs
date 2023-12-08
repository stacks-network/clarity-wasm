use clar2wasm::tools::evaluate;
use clarity::vm::types::ListTypeData;
use proptest::proptest;

use crate::property_value::PropValue;

use proptest::prelude::ProptestConfig;

use clarity::vm::types::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1, .. ProptestConfig::default()
    })]
    #[test]
    fn generated_value_is_syntactically_correct(val in PropValue::from_type(
        ListTypeData::new_list(
            TupleTypeSignature::try_from(vec![("a".into(), TypeSignature::IntType), ("b".into(), TypeSignature::SequenceType(SequenceSubtype::BufferType(5u32.try_into().unwrap())))]).unwrap().into(), 5).unwrap().into())) {
        assert_eq!(
            evaluate(dbg!(&val.to_string())),
            Some(val.into())
        )
    }
}

#[test]
fn foo() {
    let s = r#"(tuple (MjFINdZuXGsaTy 44844762225723555664080974073803592432) (PaNQkDUqmQL false) (UZMSiq (tuple (AoJVgldLjSLqkT -92366095239868827590743074977731212396) (CDOTyucGWoiMH true) (InGSd u302090107491055085799280316876908079359) (OhyIooasqgKMmz false) (ZAyLk "=ea3RwS5CNR4}kNYnA;DU;nk`MwjtjU[K:}WSsuWwjiC3gOtnm@w:[ubKjZU") (iajBJ 0x70d618a05e8ef0c5113bd7754e4b34f245f92af40d4cad376a2b3317c4fa94ae55d2c2b4cedf27cc070b9e9b91163dfae85ac94d040e78056b442ed9ba34c8e0) (jWBWiDYekB 73422001366518305422524108798144725629) (lXXXESmdDLXW "`A=cV`n@AXbSk>VYvWhRC[u}5lmG{xA8ZNVLfDuihttDf[hdtWO2L>gEuFqr7PncQF9LZRLYX}DydhcH}gQdZQm3TSkGA<<]f3lHq?NEr6AlATCe}3^C|UnX2FAdtMu]q7A>in8"))) (jQTxSULMyNj 0xe1a6847bfbf1a4fe3d0c379612902f038c40ecb9160cc9fb59ea18d8ad1a1e341c6c1e81f721f2deb749062bc70776c2ba925959ef) (uleoHioK u176459163320789576544649625075904497700) (xCgwQRhJnWyPfzqG 0xfc1b9d7770aab4f4561696dbef755dd69f5033b05a2d708ab57789b8804c04e538ebcd17db0f2812368a9b1bc30a2073f6b9e222f31076009fee4a04130a2bb32250eee7e60568d6f3e24330bea73ea5c09fbe0cfe7c3dfe1ac903e951ade632e99553a8c81b706cac425a6a6d108d56af7f61cd0fd27059afd70a14bc3968444259a83b3f074fac3d05008751a4111905eb088205b451187a553fcd13c4029b4caf7abfe7b90f2ec1f528604e307d03ec81eb1557d56652b76a3e25e93a70a16944efa3ce6c49c420fa1aea1e386da431589414919db827))"#;
    let v = evaluate(s).unwrap();
    let p: PropValue = v.into();
    assert_eq!(s, &p.to_string());
    // evaluate(&format!("{p}")).unwrap();
}

#[test]
fn foo2() {
    let s = "(list (tuple (a 148987255394482843142261275651954923681) (b 0x07fa6db2f2)) (tuple (a 10625222246108328138550657607965884282) (b 0x2b6d48b605)))";
    let _v = evaluate(s);
}
