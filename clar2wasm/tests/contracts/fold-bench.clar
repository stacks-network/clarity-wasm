(define-private (add-square (x int) (y int))
    (+ (* x x) y)
)

(define-public (fold-add-square (l (list 8192 int)) (init int))
    (ok (fold add-square l init))
)