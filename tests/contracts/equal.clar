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

(define-public (buf-equal)
    (ok (is-eq 0x0102 0x0102))
)

(define-public (buf-equal-2)
    (ok (is-eq 0x0102 0x0102 0x0102))
)

(define-public (buf-unequal)
    (ok (is-eq 0x0102 0x0103))
)

(define-public (buf-unequal-2)
    (ok (is-eq 0x0102 0x010203))
)

(define-public (buf-unequal-3)
    (ok (is-eq 0x01 0x01 0x02))
)

(define-public (str-ascii-equal)
    (ok (is-eq "hello" "hello"))
)

(define-public (str-ascii-unequal)
    (ok (is-eq "hello" "world"))
)

(define-public (principal-equal)
    (ok (is-eq 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (principal-unequal)
    (ok (is-eq 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR))
)

(define-public (call-principal-equal)
    (ok (is-eq 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.foo 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.foo))
)

(define-public (call-principal-unequal)
    (ok (is-eq 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.foo 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.bar))
)

(define-public (call-principal-unequal-2)
    (ok (is-eq 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.foo 'SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR.foo))
)

(define-public (call-optional-equal)
    (ok (is-eq (some 1) (some 1)))
)

(define-public (call-optional-unequal)
    (ok (is-eq (some 0x01) (some 0x02)))
)

(define-public (call-response-ok-equal)
    (ok (is-eq (ok 0) (ok 0)))
)

(define-public (call-response-err-equal)
    (ok (is-eq (err 0x010203) (err 0x010203)))
)

(define-public (call-response-ok-err-unequal)
    (ok (is-eq (ok 42) (err "forty-two")))
)

(define-public (call-response-ok-unequal)
    (ok (is-eq (ok u5) (ok u55)))
)

(define-public (call-response-err-unequal)
    (ok (is-eq (err 0x123456) (err 0xabcdef)))
)