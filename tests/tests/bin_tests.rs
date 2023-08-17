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

    for file in std::fs::read_dir("./contracts/").unwrap() {
        let file = file.unwrap();
        if file.path().extension().unwrap() != "clar" {
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
