(define-public (ok-truthy)
    (ok (is-ok (ok 1)))
)

(define-public (ok-falsy)
    (ok (is-ok (err 1)))
)
