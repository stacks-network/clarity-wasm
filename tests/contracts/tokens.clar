(define-fungible-token foo)
(define-fungible-token bar u1000000)
(define-non-fungible-token baz uint)

(define-public (foo-get-supply-0)
  (ok (ft-get-supply foo))
)

(define-public (foo-mint)
  (ft-mint? foo u1000 tx-sender)
)

(define-public (foo-mint-0)
  (ft-mint? foo u0 tx-sender)
)

(define-public (bar-mint-too-many)
  (ft-mint? bar u1000001 tx-sender)
)

(define-public (bar-mint-too-many-2)
  (begin
    (unwrap-panic (ft-mint? bar u5555555 tx-sender))
    (ft-mint? bar u5555555 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (ft-balance-0)
  (ok (ft-get-balance foo tx-sender))
)

(define-public (ft-balance-10)
  (begin
    (unwrap-panic (ft-mint? foo u10 tx-sender))
    (ok (ft-get-balance foo tx-sender))
  )
)

(define-public (ft-burn-unowned)
  (ft-burn? foo u10 tx-sender)
)

(define-public (ft-burn-0)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-burn? foo u0 tx-sender)
  )
)

(define-public (ft-burn-ok)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-burn? foo u10 tx-sender)
  )
)

(define-public (ft-burn-too-many)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-burn? foo u2000 tx-sender)
  )
)

(define-public (ft-burn-other)
  (begin
    (unwrap-panic (ft-mint? foo u1000 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (ft-burn? foo u200 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (ft-transfer-unowned)
  (ft-transfer? foo u10 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (ft-transfer-0)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-transfer? foo u0 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (ft-transfer-ok)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-transfer? foo u10 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (ft-transfer-too-many)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-transfer? foo u2000 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (ft-transfer-other)
  (begin
    (unwrap-panic (ft-mint? foo u1000 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (ft-transfer? foo u200 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM tx-sender)
  )
)

(define-public (ft-transfer-self)
  (begin
    (unwrap-panic (ft-mint? foo u1000 tx-sender))
    (ft-transfer? foo u10 tx-sender tx-sender)
  )
)

(define-public (nft-mint)
  (nft-mint? baz u0 tx-sender)
)

(define-public (nft-mint-other)
  (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (nft-mint-duplicate)
  (begin
    (unwrap-panic (nft-mint? baz u0 tx-sender))
    (nft-mint? baz u0 tx-sender)
  )
)

(define-public (nft-get-owner)
  (begin
    (unwrap-panic (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (ok (nft-get-owner? baz u0))
  )
)

(define-public (nft-get-owner-unowned)
  (ok (nft-get-owner? baz u0))
)

(define-public (nft-burn)
  (begin
    (unwrap-panic (nft-mint? baz u0 tx-sender))
    (nft-burn? baz u0 tx-sender)
  )
)

(define-public (nft-burn-unowned)
  (nft-burn? baz u0 tx-sender)
)

(define-public (nft-burn-other)
  (begin
    (unwrap-panic (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (nft-burn? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (nft-burn-wrong)
  (begin
    (unwrap-panic (nft-mint? baz u0 tx-sender))
    (nft-burn? baz u1 tx-sender)
  )
)


(define-public (nft-transfer-does-not-exist)
  (nft-transfer? baz u0 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
)

(define-public (nft-transfer-ok)
  (begin
    (unwrap-panic (nft-mint? baz u0 tx-sender))
    (nft-transfer? baz u0 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (nft-transfer-wrong)
  (begin
    (unwrap-panic (nft-mint? baz u0 tx-sender))
    (nft-transfer? baz u1 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (nft-transfer-not-owner)
  (begin
    (unwrap-panic (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (nft-transfer? baz u0 tx-sender 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
  )
)

(define-public (nft-transfer-self)
  (begin
    (unwrap-panic (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (nft-transfer? baz u0 tx-sender tx-sender)
  )
)

(define-public (nft-transfer-other)
  (begin
    (unwrap-panic (nft-mint? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
    (nft-transfer? baz u0 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM tx-sender)
  )
)
