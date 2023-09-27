(define-private (try-opt (x (optional uint)))
  (unwrap-panic x)
)

(define-private (try-res (x (response uint uint)))
  (unwrap-panic x)
)

(define-public (unwrap-some)
  (ok (try-opt (some u1)))
)

(define-public (unwrap-none)
  (ok (try-opt none))
)

(define-public (unwrap-ok)
  (ok (try-res (ok u1)))
)

(define-public (unwrap-error)
  (ok (try-res (err u1)))
)