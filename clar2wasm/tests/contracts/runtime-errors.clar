;; runtime error 0
(define-public (overflow-error)
    (ok (pow 12345678987654321 100))
)

;; runtime error 1
(define-public (underflow-error)
    (ok (- u2 u10))
)

;; runtime error 2
(define-public (division-by-zero-error)
    (ok (/ 42 0))
)

;; runtime error 3
(define-public (log2-argument-error)
    (ok (log2 -8))
)

;; runtime error 4
(define-public (square-root-argument-error)
    (ok (sqrti -3))
)

;; runtime error 5
(define-public (unwrap-error)
    (ok (unwrap-panic (get-block-info? id-header-hash u13)))
)

;; runtime error 8
(define-public (power-argument-error)
    (ok (pow 2 -3))
)
