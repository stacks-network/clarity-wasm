(define-public (check-sender)
  (ok (as-contract tx-sender))
)

(define-public (check-caller)
  (ok (as-contract contract-caller))
)