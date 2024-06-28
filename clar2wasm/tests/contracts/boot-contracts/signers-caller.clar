
(define-public (get-signer-by-index)
  (contract-call? .signers get-signer-by-index u1 u3)
)

(define-public (stackerdb-get-config)
  (contract-call? .signers stackerdb-get-config)
)

(define-public (get-last-set-cycle)
  (contract-call? .signers get-last-set-cycle)
)
