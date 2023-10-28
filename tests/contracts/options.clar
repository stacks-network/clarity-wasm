(define-public (some-truthy)
    (ok (is-some (some 1)))
)

(define-public (some-falsy)
    (ok (is-some (element-at? (list 1 2 3 4 5) u99)))
)

(define-public (none-truthy)
    (ok (is-none (element-at? (list 1 2 5) u5)))
)

(define-public (none-falsy)
    (ok (is-none (some 1)))
)
