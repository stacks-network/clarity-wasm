(define-public (ok-truthy)
    (ok (is-ok (ok 1)))
)

(define-public (ok-falsy)
    (ok (is-ok (err 1)))
)

(define-public (err-truthy)
    (ok (is-err (err 1)))
)

(define-public (err-falsy)
    (ok (is-err (ok 1)))
)
