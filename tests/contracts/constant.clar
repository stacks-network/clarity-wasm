(define-constant small-int 1)
(define-public (get-int-constant)
  (ok small-int)
)

(define-constant large-uint u338770000845734292516042252062085074415)
(define-public (get-large-uint-constant)
  (ok large-uint)
)

(define-constant string "hello world")
(define-public (get-string-constant)
  (ok string)
)

(define-constant string-utf8 u"hello world\u{1F98A}")
(define-public (get-string-utf8-constant)
  (ok string-utf8)
)

(define-constant bytes 0x12345678)
(define-public (get-bytes-constant)
  (ok bytes)
)