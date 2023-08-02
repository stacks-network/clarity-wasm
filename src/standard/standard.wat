;; This module contains a standard library for Clarity, defining Clarity's
;; builtins, to be called from the generated Wasm code.
(module
    (type (;0;) (func))
    (type (;1;) (func (param i64 i64 i64 i64) (result i64 i64)))

    (func $panic (type 0)
        ;; TODO: Implement panic
        unreachable
    )

    ;; This function can be used to add either signed or unsigned integers
    (func $add128 (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $sum_lo i64)
        (local $sum_hi i64)
        (local $carry i64)

        ;; Add the lower 64 bits
        (local.set $sum_lo (i64.add (local.get $a_lo) (local.get $b_lo)))

        ;; Check for carry
        (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $sum_lo) (local.get $a_lo))))

        ;; Add the upper 64 bits, accounting for any carry from the lower bits
        (local.set $sum_hi (i64.add (i64.add (local.get $a_hi) (local.get $b_hi)) (local.get $carry)))

        ;; Return the result
        (return (local.get $sum_hi) (local.get $sum_lo))
    )

    (func $add-int (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $sum_hi i64)
        (local $sum_lo i64)

        (local.get $a_hi)
        (local.get $a_lo)
        (local.get $b_hi)
        (local.get $b_lo)
        (call $add128)

        (local.set $sum_lo)
        (local.set $sum_hi)

        ;; Check for overflow and underflow
        (if (i64.eq (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $b_hi) (i64.const 63))) ;; if a and b have the same sign
            (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $sum_hi) (i64.const 63))) ;; and the result has a different sign
                (call $panic)
            )
        )

        ;; Return the result
        (return (local.get $sum_hi) (local.get $sum_lo))
    )

    (func $add-uint (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $sum_hi i64)
        (local $sum_lo i64)

        (local.get $a_hi)
        (local.get $a_lo)
        (local.get $b_hi)
        (local.get $b_lo)
        (call $add128)

        (local.set $sum_lo)
        (local.set $sum_hi)

        ;; Check for overflow
        (if (i64.lt_u (local.get $sum_hi) (local.get $a_hi))
            (call $panic)
        )

        ;; Return the result
        (return (local.get $sum_hi) (local.get $sum_lo))
    )

    ;; This function can be used to subtract either signed or unsigned integers
    (func $sub-int128 (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $borrow i64)
        (local $diff_lo i64)
        (local $diff_hi i64)

        ;; Calculate borrow
        (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $a_lo) (local.get $b_lo))))

        ;; Calculate diff_lo
        (local.set $diff_lo (i64.sub (local.get $a_lo) (local.get $b_lo)))

        ;; Calculate diff_hi
        (local.set $diff_hi (i64.sub (i64.sub (local.get $a_hi) (local.get $b_hi)) (local.get $borrow)))

        ;; Return the result
        (return (local.get $diff_hi) (local.get $diff_lo))
    )

    (func $sub-int (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $diff_hi i64)
        (local $diff_lo i64)

        (local.get $a_hi)
        (local.get $a_lo)
        (local.get $b_hi)
        (local.get $b_lo)
        (call $sub-int128)

        (local.set $diff_lo)
        (local.set $diff_hi)

        ;; Check for overflow and underflow
        (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $b_hi) (i64.const 63))) ;; if a and b have different signs
            (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $diff_hi) (i64.const 63))) ;; and the result has a different sign from a
                (call $panic)
            )
        )

        ;; Return the result
        (return (local.get $diff_hi) (local.get $diff_lo))
    )

    (func $sub-uint (type 1) (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $diff_hi i64)
        (local $diff_lo i64)

        (local.get $a_hi)
        (local.get $a_lo)
        (local.get $b_hi)
        (local.get $b_lo)
        (call $sub-int128)

        (local.set $diff_lo)
        (local.set $diff_hi)

        ;; Check for underflow
        (if (i64.gt_u (local.get $diff_hi) (local.get $a_hi))
            (then
                (call $panic)
            )
        )

        ;; Return the result
        (return (local.get $diff_hi) (local.get $diff_lo))
    )

    (export "add-int" (func $add-int))
    (export "add-uint" (func $add-uint))
    (export "sub-int" (func $sub-int))
    (export "sub-uint" (func $sub-uint))
)