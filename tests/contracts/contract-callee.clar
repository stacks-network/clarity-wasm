(define-public (no-args)
  (ok u42)
)

(define-public (one-simple-arg (x int))
  (ok x)
)

(define-public (one-arg (x (string-ascii 16)))
  (ok x)
)

(define-public (two-simple-args (x int) (y int))
  (ok (+ x y))
)

(define-public (two-args (x (string-ascii 16)) (y (string-ascii 16)))
  (ok (concat x y))
)