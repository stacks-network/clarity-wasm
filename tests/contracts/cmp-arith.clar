(define-public (less-uint)
    (ok (< u1 u2))
)

(define-public (greater-int)
    (ok (> -1000 -2000))
)

(define-public (less-or-equal-uint)
    (ok (<= u42 u42))
)

(define-public (greater-or-equal-int)
    (ok (>= 42 -5130))
)