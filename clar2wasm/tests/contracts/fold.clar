(define-private (sub (x int) (y int))
    (- x y)
)

(define-public (fold-sub)
    (ok (fold sub (list 1 2 3 4) 0))
)
