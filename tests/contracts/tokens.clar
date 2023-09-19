(define-public (test-stx-get-balance)
  (ok (stx-get-balance 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (test-stx-account)
  (ok (stx-account 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (test-stx-burn-ok)
  (stx-burn? u100 'S1G2081040G2081040G2081040G208105NK8PE5)
)

(define-public (test-stx-burn-err1)
  ;; not enough balance
  (stx-burn? u5000000000 'S1G2081040G2081040G2081040G208105NK8PE5)
)

(define-public (test-stx-burn-err3)
  ;; non-positive amount
  (stx-burn? u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (test-stx-burn-err4)
  ;; sender is not tx-sender
  (stx-burn? u100 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)
