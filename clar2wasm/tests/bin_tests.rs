use std::ffi::OsStr;

#[test]
fn test_clar2wasm_no_args() {
    assert_cmd::Command::cargo_bin("clar2wasm")
        .unwrap()
        .assert()
        .stderr(predicates::str::contains("Usage:"))
        .failure();
}

#[test]
fn test_clar2wasm_with_input() {
    let temp = assert_fs::TempDir::new().unwrap();

    for file in std::fs::read_dir("./tests/contracts/").unwrap() {
        let file = file.unwrap();
        if file.path().extension().unwrap_or(OsStr::new("")) != "clar" {
            continue;
        }
        if file.path().file_name().unwrap() == "bns.clar" {
            // bns.clar uses `block-height` which is not supported in clarity 3
            // the clar2wasm bin should accept a ClarityVersion flag
            continue;
        }
        let outfile = temp.join(
            file.file_name()
                .into_string()
                .unwrap()
                .replace(".clar", ".wasm"),
        );

        assert_cmd::Command::cargo_bin("clar2wasm")
            .unwrap()
            .arg(file.path())
            .arg("-o")
            .arg(&outfile)
            .assert()
            .success();

        wasmparser::validate(&std::fs::read(outfile).unwrap()).unwrap();
    }

    temp.close().unwrap();
}
