(define-data-var something int 123)

(define-public (simple)
    (ok (var-get something))
)