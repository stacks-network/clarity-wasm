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

;; TODO: Enable this test once `try` is implemented
;; (define-public (bar-mint-too-many-2)
;;   (begin
;;     (try! (ft-mint? bar u5555555 tx-sender))
;;     (ft-mint? bar u5555555 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)
;;   )
;; )
