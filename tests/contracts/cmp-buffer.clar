(define-public (less-buffer)
    (ok (< 0x0102 0x0103))
)

(define-public (less-string-ascii)
    (ok (< "hello" "world"))
)

(define-public (greater-buffer)
    (ok (> 0x0102 0x0103))
)

(define-public (greater-string-ascii)
    (ok (> "hello" "world"))
)

(define-public (less-or-equal-buffer)
    (ok (<= 0x0102 0x0103))
)

(define-public (less-or-equal-string-ascii)
    (ok (<= "hello" "world"))
)

(define-public (greater-or-equal-buffer)
    (ok (>= 0x0102 0x0103))
)

(define-public (greater-or-equal-string-ascii)
    (ok (>= "hello" "world"))
)