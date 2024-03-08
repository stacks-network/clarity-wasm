(defun rust-test ()
  "Test using `cargo test`."
  (interactive)
  (compile "cargo test -j4"))
