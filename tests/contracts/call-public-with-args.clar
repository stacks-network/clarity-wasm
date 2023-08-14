(define-public (simple (a int) (b int))
  (ok (+ a b))
)

(define-public (call-it)
  (simple 1 2)
)
