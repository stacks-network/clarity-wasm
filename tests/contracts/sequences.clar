(define-public (list-append)
  (ok (append (list 1 2) 3))
)

(define-public (list-append-strings)
  (ok (append (list "hello" "world") "!"))
)

(define-public (list-append-empty)
  (ok (append (list) true))
)

(define-public (list-as-max-len-some)
  (ok (as-max-len? (list 1 2) u4))
)

(define-public (list-as-max-len-none)
  (ok (as-max-len? (list 1 2) u1))
)

(define-public (list-as-max-len-empty)
  (ok (as-max-len? (list) u8))
)

(define-public (string-as-max-len)
  (ok (as-max-len? "hello" u8))
)

(define-public (buffer-as-max-len)
  (ok (as-max-len? 0x123456 u4))
)

(define-public (list-concat)
  (ok (concat (list 1 2) (list 3 4)))
)

(define-public (string-concat)
  (ok (concat "hello" " world"))
)

(define-public (buffer-concat)
  (ok (concat 0x123456 0x789abc))
)