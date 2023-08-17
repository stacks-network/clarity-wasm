(define-private (add (x int) (y int))
    (+ x y)
)

(define-public (fold-add)
    (ok (fold add (list 1 2 3 4) 0))
)
