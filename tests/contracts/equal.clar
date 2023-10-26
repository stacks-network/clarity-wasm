(define-public (one-int-equal)
    (ok (is-eq 42))
)

(define-public (one-uint-equal)
    (ok (is-eq u99))
)

(define-public (two-zero-int-equal)
    (ok (is-eq 0 0))
)

(define-public (two-zero-uint-equal)
    (ok (is-eq u0 u0))
)

(define-public (two-int-equal)
    (ok (is-eq 42 42))
)

(define-public (two-uint-equal)
    (ok (is-eq u11 u11))
)

(define-public (two-int-unequal)
    (ok (is-eq 43 88))
)

(define-public (two-uint-unequal)
    (ok (is-eq u33 u123))
)

(define-public (int-equal)
    (ok (is-eq 12 12 12 12 12 12 12 12))
)

(define-public (uint-equal)
    (ok (is-eq u37 u37 u37 u37))
)

(define-public (int-unequal)
    (ok (is-eq 12 12 15 12 12 12 12 12))
)

(define-public (int-unequal-2)
    (ok (is-eq 12 13 15 12 12 12 12 12))
)

(define-public (uint-unequal)
    (ok (is-eq u3 u3 u3 u5 u6 u3 u3))
)

(define-public (uint-unequal-2)
    (ok (is-eq u43 u43 u54 u56 u43 u43))
)
