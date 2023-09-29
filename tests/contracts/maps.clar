(define-map scores uint uint)

(define-public (test-map-insert)
  (ok (map-insert scores u1 u2))
)

(define-public (test-map-insert-exists)
  (begin
    (map-insert scores u2 u1)
    (ok (map-insert scores u2 u2))
  )
)

(define-public (test-map-set)
  (ok (map-set scores u3 u2))
)

(define-public (test-map-set-exists)
  (begin
    (map-set scores u4 u1)
    (ok (map-set scores u4 u2))
  )
)

(define-public (test-map-get-insert)
  (begin
    (map-insert scores u5 u2)
    (ok (map-get? scores u5))
  )
)

(define-public (test-map-get-insert-exists)
  (begin
    (map-insert scores u6 u1)
    (map-insert scores u6 u2)
    (ok (map-get? scores u6))
  )
)

(define-public (test-map-get-set)
  (begin
    (map-set scores u7 u2)
    (ok (map-get? scores u7))
  )
)

(define-public (test-map-get-set-exists)
  (begin
    (map-set scores u8 u1)
    (map-set scores u8 u2)
    (ok (map-get? scores u8))
  )
)

(define-public (test-map-get-none)
  (ok (map-get? scores u9))
)

(define-public (test-map-delete)
  (begin
    (map-insert scores u10 u2)
    (ok (map-delete scores u10))
  )
)

(define-public (test-map-delete-none)
  (ok (map-delete scores u11))
)

(define-public (test-map-delete-get)
  (begin
    (map-insert scores u12 u2)
    (map-delete scores u12)
    (ok (map-get? scores u12))
  )
)
