(define-constant BAR 42)

(define-public (get-bar)
    (ok BAR))

(define-private (im-a-private-func) (get-bar))
