(define-public (check-sender)
  (ok (as-contract tx-sender))
)

(define-public (check-caller)
  (ok (as-contract contract-caller))
)

;; Make sure that `as-contract` doesn't leak outside of it's scope
(define-public (check-sender-after-as-contract)
  (begin
    (as-contract 42)
    (ok tx-sender)
  )
)

;; Make sure that `as-contract` doesn't leak outside of it's scope
(define-public (check-caller-after-as-contract)
  (begin
    (as-contract 42)
    (ok contract-caller)
  )
)