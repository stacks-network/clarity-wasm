(define-public (no-args)
  (contract-call? .contract-callee no-args)
)

(define-public (one-simple-arg)
  (contract-call? .contract-callee one-simple-arg 17)
)

(define-public (one-arg)
  (contract-call? .contract-callee one-arg "hello")
)

(define-public (two-simple-args)
  (contract-call? .contract-callee two-simple-args 17 42)
)

(define-public (two-args)
  (contract-call? .contract-callee two-args "hello " "world")
)