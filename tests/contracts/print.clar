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

(define-public (print-true)
  (ok (print true))
)

(define-public (print-false)
  (ok (print false))
)

(define-public (print-none)
  (ok (print none))
)

(define-public (print-some)
  (ok (print (some 42)))
)

(define-public (print-list)
  (ok (print (list 1 2 3)))
)

(define-public (print-list-principals)
  (ok (print (list 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.contract)))
)

(define-public (print-list-empty)
  (ok (print (list)))
)

(define-public (print-buffer)
  (ok (print 0xdeadbeef))
)

(define-public (print-buffer-empty)
  (ok (print 0x))
)

(define-data-var my-data uint u0)
(define-private (increment)
  (var-set my-data (+ (var-get my-data) u1))
)

(define-public (print-side-effect)
  (begin
    (print (increment))
    (ok (var-get my-data))
  )
)

(define-public (print-string-ascii)
  (ok (print "hello world"))
)

(define-public (print-string-ascii-empty)
  (ok (print ""))
)

(define-public (print-tuple)
  (ok (print {key1: 1, key2: true}))
)