
(define-data-var abc int 42)

(define-public (foo)
  (ok
    (at-block
      (unwrap-panic (get-block-info? id-header-hash u0))
      (var-get abc))))
