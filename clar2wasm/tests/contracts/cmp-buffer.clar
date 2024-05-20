(define-public (less-buffer)
    (ok (< 0x0102 0x0103))
)

(define-public (less-string-ascii)
    (ok (< "hello" "world"))
)

(define-public (less-string-utf8-a)
    (ok (< u"\u{0380}" u"Z")) ;; false
)

(define-public (less-string-utf8-b)
    (ok (< u"\u{5a}" u"\u{5b}")) ;; true
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

(define-public (less-or-equal-string-utf8)
    (ok (<= u"\u{5a}" u"Z")) ;; true
)

(define-public (greater-or-equal-buffer)
    (ok (>= 0x0102 0x0103))
)

(define-public (greater-or-equal-string-ascii)
    (ok (>= "hello" "world"))
)

(define-public (greater-or-equal-string-utf8)
    (ok (>= u"\u{5a}" u"Z")) ;; true
)

(define-public (less-buffer-diff-len)
    (ok (< 0x01 0x010203))
)

(define-public (less-string-ascii-diff-len)
    (ok (< "Lorem ipsum" "Lorem ipsum dolor sit amet"))
)

(define-public (less-string-utf8-diff-len)
    (ok (< u"stacks" u"st\u{c3a4}cks")) ;; true
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