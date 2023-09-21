;; (print 0x0123456789abcdef)
;; (print "hello world")

;;(define-public (print-hello)
;;  (begin
;;    (print 12345)
;;    (ok true)
;;  )
;;)

(define-public (print-hello)
  (ok (print 12345))
)
