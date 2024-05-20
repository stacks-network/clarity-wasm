(define-public (non-existent)
  (ok (get-block-info? time u9999999))
)

(define-public (get-burnchain-header-hash)
  (ok (get-block-info? burnchain-header-hash u0))
)

(define-public (get-id-header-hash)
  (ok (get-block-info? id-header-hash u0))
)

(define-public (get-header-hash)
  (ok (get-block-info? header-hash u0))
)

(define-public (get-miner-address)
  (ok (get-block-info? miner-address u0))
)

(define-public (get-time)
  (ok (get-block-info? time u0))
)

(define-public (get-block-reward)
  (ok (get-block-info? block-reward u0))
)

(define-public (get-miner-spend-total)
  (ok (get-block-info? miner-spend-total u0))
)

(define-public (get-miner-spend-winner)
  (ok (get-block-info? miner-spend-winner u0))
)
