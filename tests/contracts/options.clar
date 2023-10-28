(define-public (some-truthy)
    (ok (is-some (some 1)))
)

(define-public (some-falsy)
    (ok (is-some (element-at? (list 1 2 3 4 5) u99)))
)

