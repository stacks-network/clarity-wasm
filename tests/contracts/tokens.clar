(define-public (test-stx-get-balance)
  (ok (stx-get-balance 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (test-stx-account)
  (ok (stx-account 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (test-stx-burn-ok)
  ;; success
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

(define-public (test-stx-transfer-ok)
  ;; success, no memo
  (stx-transfer? u100 'S1G2081040G2081040G2081040G208105NK8PE5 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (test-stx-transfer-memo-ok)
  ;; success, memo
  (stx-transfer-memo? u100 'S1G2081040G2081040G2081040G208105NK8PE5 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 0x12345678)
)

(define-public (test-stx-transfer-err1)
  ;; not enough balance
  (stx-transfer? u5000000000 'S1G2081040G2081040G2081040G208105NK8PE5 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (test-stx-transfer-err2)
  ;; sender is recipient
  (stx-transfer? u5000000000 tx-sender 'S1G2081040G2081040G2081040G208105NK8PE5)
)

(define-public (test-stx-transfer-err3)
  ;; non-positive amount
  (stx-transfer? u0 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (test-stx-transfer-err4)
  ;; sender is not tx-sender
  (stx-transfer? u100 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM tx-sender)
)
