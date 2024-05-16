(define-public (default-to-value)
    (ok (default-to 767 none))
)

(define-public (default-to-some)
    (ok (default-to 767 (some 42)))
)

(define-public (default-to-some-string)
    (ok (default-to "a" (element-at? "Clarity" u0)))
)

(define-public (default-to-list)
    (ok (default-to (list 1 2 3) none))
)
