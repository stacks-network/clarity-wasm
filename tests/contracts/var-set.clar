(define-data-var something int 123)

(define-public (simple)
    (begin
        (var-set something 5368002525449479521366)
        (ok (var-get something))
    )
)