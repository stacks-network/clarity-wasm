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

(define-public (less-buffer-diff-len)
    (ok (< 0x01 0x010203))
)

(define-public (less-string-ascii-diff-len)
    (ok (< "Lorem ipsum" "Lorem ipsum dolor sit amet"))
)

(define-public (greater-buffer-diff-len)
    (ok (> 0x01 0x010203))
)

(define-public (greater-string-ascii-diff-len)
    (ok (> "Lorem ipsum" "Lorem ipsum dolor sit amet"))
)

(define-public (less-or-equal-buffer-diff-len)
    (ok (<= 0x01 0x010203))
)

(define-public (less-or-equal-string-ascii-diff-len)
    (ok (<= "Lorem ipsum" "Lorem ipsum dolor sit amet"))
)

(define-public (greater-or-equal-buffer-diff-len)
    (ok (>= 0x01 0x010203))
)

(define-public (greater-or-equal-string-ascii-diff-len)
    (ok (>= "Lorem ipsum" "Lorem ipsum dolor sit amet"))
)

(define-public (less-same-buffer)
    (ok (< 0x01 0x01))
)

(define-public (less-same-string-ascii)
    (ok (< "Lorem ipsum" "Lorem ipsum"))
)

(define-public (greater-same-buffer)
    (ok (> 0x01 0x01))
)

(define-public (greater-same-string-ascii)
    (ok (> "Lorem ipsum" "Lorem ipsum"))
)

(define-public (less-or-equal-same-buffer)
    (ok (<= 0x01 0x01))
)

(define-public (less-or-equal-same-string-ascii)
    (ok (<= "Lorem ipsum" "Lorem ipsum"))
)

(define-public (greater-or-equal-same-buffer)
    (ok (>= 0x01 0x01))
)

(define-public (greater-or-equal-same-string-ascii)
    (ok (>= "Lorem ipsum" "Lorem ipsum"))
)