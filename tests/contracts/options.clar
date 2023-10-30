(define-public (some-truthy)
    (ok (is-some (some 1)))
)

(define-public (some-falsy)
    (ok (is-some none))
)

(define-public (none-truthy)
    (ok (is-none none))
)

(define-public (none-falsy)
    (ok (is-none (some 1)))
)
