(define-public (test-to-int)
    (ok (to-int u42)))

;; type `i128` range is
;; -170141183460469231731687303715884105728 to 170141183460469231731687303715884105727
(define-public (test-to-int-limit)
    (ok (to-int u170141183460469231731687303715884105727)))

(define-public (test-to-int-out-of-boundary)
    (ok (to-int u170141183460469231731687303715884105728)))

(define-public (test-to-uint)
    (ok (to-uint 767)))

(define-public (test-to-uint-error)
    (ok (to-uint -47)))
