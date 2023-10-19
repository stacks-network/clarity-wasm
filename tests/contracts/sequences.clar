(define-public (list-append)
  (ok (append (list 1 2) 3))
)

(define-public (list-append-strings)
  (ok (append (list "hello" "world") "!"))
)