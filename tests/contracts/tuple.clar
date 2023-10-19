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

(define-public (get-first)
  (ok (get a {a: 42, b: false}))
)

(define-public (get-last)
  (ok (get quote {
    a: 42,
    b: false,
    quote: "Great ideas often receive violent opposition from mediocre minds."
  }))
)

(define-public (get-only)
  (ok (get only {only: 0x12345678}))
)
