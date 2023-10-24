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

(define-public (list-len)
  (ok (len (list 1 2 3)))
)

(define-public (string-len)
  (ok (len "sup"))
)

(define-public (buffer-len)
  (ok (len 0x123456))
)

(define-public (list-len-0)
  (ok (len (list)))
)

(define-public (string-len-0)
  (ok (len ""))
)

(define-public (buffer-len-0)
  (ok (len 0x))
)

(define-public (list-element-at)
  (ok (element-at (list 1 2 3) u1))
)

(define-public (string-element-at)
  (ok (element-at "hello" u4))
)

(define-public (buffer-element-at)
  (ok (element-at 0x123456 u2))
)

(define-public (list-element-at?)
  (ok (element-at? (list 1 2 3) u1))
)

(define-public (string-element-at?)
  (ok (element-at? "hello" u4))
)

(define-public (buffer-element-at?)
  (ok (element-at? 0x123456 u2))
)

(define-public (list-element-at-none)
  (ok (element-at? (list 1 2 3) u3))
)

(define-public (string-element-at-none)
  (ok (element-at? "hello" u5))
)

(define-public (buffer-element-at-none)
  (ok (element-at? 0x123456 u3))
)

;; 18446744073709551617 == 2^64 + 1
(define-public (element-at-upper-offset)
  (ok (element-at (list 1 2 3 4 5) u18446744073709551617))
)

(define-public (list-replace-at)
  (ok (replace-at? (list 1 2 3) u1 4))
)

(define-public (string-replace-at)
  (ok (replace-at? "hello" u0 "j"))
)

(define-public (buffer-replace-at)
  (ok (replace-at? 0xfedcba9876543210 u4 0x67))
)

(define-public (list-replace-at-none)
  (ok (replace-at? (list 1 2 3) u4 4))
)

(define-public (string-replace-at-none)
  (ok (replace-at? "hello" u5 "X"))
)

(define-public (buffer-replace-at-none)
  (ok (replace-at? 0xfedcba9876543210 u123 0x67))
)