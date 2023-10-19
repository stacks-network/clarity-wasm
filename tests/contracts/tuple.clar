(define-public (simple)
  (ok {a: 1, b: u2})
)

(define-public (out-of-order)
  (ok {b: u2, a: 1})
)

(define-public (list-syntax)
  (ok (tuple (a 1) (b u2)))
)

(define-public (strings)
  (ok {one: "one", two: "two", three: "three"})
)

(define-public (nested)
  (ok {a: 1, b: {c: 2, d: 3}})
)
