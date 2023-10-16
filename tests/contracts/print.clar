(define-public (print-int)
  (ok (print 12345))
)

(define-public (print-uint)
  (ok (print u98765))
)

(define-public (print-standard-principal)
  (ok (print 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (print-contract-principal)
  (ok (print 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.foo))
)

(define-public (print-response-ok-int)
  (print (ok 12345))
)

(define-public (print-response-err-uint)
  (print (err u98765))
)

(define-public (print-response-ok-principal)
  (print (ok 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)

(define-public (print-response-err-principal)
  (print (err 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM))
)
