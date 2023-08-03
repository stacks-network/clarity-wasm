;; This module contains a standard library for Clarity, defining Clarity's
;; builtins, to be called from the generated Wasm code.
(module
    (type (;0;) (func))
    (type (;1;) (func (param i64 i64 i64 i64) (result i64 i64)))

    (func $overflow (type 0)
        ;; TODO: Implement overflow
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
                (call $overflow)
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
            (call $overflow)
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
                (call $overflow)
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
                (call $overflow)
            )
        )

        ;; Return the result
        (return (local.get $diff_hi) (local.get $diff_lo))
    )

    (func $mul-int128 (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $a0 i32)
        (local $a1 i32)
        (local $a2 i32)
        (local $a3 i32)
        (local $b0 i32)
        (local $b1 i32)
        (local $b2 i32)
        (local $b3 i32)
        (local $product0 i64)
        (local $product1 i64)
        (local $product2 i64)
        (local $product3 i64)
        (local $carry i64)
        (local $res_hi i64)
        (local $res_lo i64)

        ;; Shortcut if either a or b is zero
        (if (i32.or
                (i64.eqz (i64.or (local.get $a_hi) (local.get $a_lo)))
                (i64.eqz (i64.or (local.get $b_hi) (local.get $b_lo))))
            (return (i64.const 0) (i64.const 0))
        )

        ;; Split the operands into 32-bit chunks
        (local.set $a0 (i32.wrap_i64 (local.get $a_lo)))
        (local.set $a1 (i32.wrap_i64 (i64.shr_u (local.get $a_lo) (i64.const 32))))
        (local.set $a2 (i32.wrap_i64 (local.get $a_hi)))
        (local.set $a3 (i32.wrap_i64 (i64.shr_u (local.get $a_hi) (i64.const 32))))
        (local.set $b0 (i32.wrap_i64 (local.get $b_lo)))
        (local.set $b1 (i32.wrap_i64 (i64.shr_u (local.get $b_lo) (i64.const 32))))
        (local.set $b2 (i32.wrap_i64 (local.get $b_hi)))
        (local.set $b3 (i32.wrap_i64 (i64.shr_u (local.get $b_hi) (i64.const 32))))

        ;; Do long multiplication over the chunks
        ;; Result = a0b0 + 
        ;;         (a1b0 + a0b1) << 32 +
        ;;         (a2b0 + a1b1 + a0b2) << 64 +
        ;;         (a3b0 + a2b1 + a1b2 + a0b3) << 96
        ;; The remaining terms are discarded because they are too large to fit in 128 bits
        ;;         (a3b1 + a2b2 + a1b3) << 128 +
        ;;         (a3b2 + a2b3) << 160 +
        ;;         a3b3 << 192
        ;; We would need to make sure these are 0 or report overflow if they are not
        
        ;; a0b0
        (local.set $res_lo (i64.mul (i64.extend_i32_u (local.get $a0)) (i64.extend_i32_u (local.get $b0))))

        ;; (a1b0 + a0b1) << 32
        ;; a1b0
        (local.set $product0 (i64.mul (i64.extend_i32_u (local.get $a1)) (i64.extend_i32_u (local.get $b0))))
        ;; a0b1
        (local.set $product1 (i64.mul (i64.extend_i32_u (local.get $a1)) (i64.extend_i32_u (local.get $b0))))
        ;; a1b0 + a0b1
        (local.set $product0 (i64.add (local.get $product0) (local.get $product1)))
        ;; check for carry
        (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $product0) (local.get $product1))))
        ;; a1b0 + a0b1 << 32
        (local.set $res_hi (i64.shr_u (local.get $product0) (i64.const 32)))
        (local.set $res_hi (i64.add (local.get $res_hi) (i64.shl (local.get $carry) (i64.const 32))))
        (local.set $res_lo (i64.add (local.get $res_lo) (i64.shl (local.get $product0) (i64.const 32))))
        ;; check for carry
        (local.set $carry (i64.extend_i32_u (i64.lt_u (local.get $res_lo) (local.get $product0))))
        (local.set $res_hi (i64.add (local.get $res_hi) (local.get $carry)))

        ;; (a2b0 + a1b1 + a0b2) << 64
        ;; a2b0
        (local.set $product0 (i64.mul (i64.extend_i32_u (local.get $a2)) (i64.extend_i32_u (local.get $b0))))
        ;; a1b1
        (local.set $product1 (i64.mul (i64.extend_i32_u (local.get $a1)) (i64.extend_i32_u (local.get $b1))))
        ;; a0b2
        (local.set $product2 (i64.mul (i64.extend_i32_u (local.get $a0)) (i64.extend_i32_u (local.get $b2))))
        ;; a2b0 + a1b1 + a0b2
        (local.set $product0 (i64.add (local.get $product0) (local.get $product1)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product1))
            (call $overflow)
        )
        (local.set $product0 (i64.add (local.get $product0) (local.get $product2)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product2))
            (call $overflow)
        )
        ;; res_hi += (a2b0 + a1b1 + a0b2)
        (local.set $res_hi (i64.add (local.get $res_hi) (local.get $product0)))
        ;; check for carry
        (if (i64.lt_u (local.get $res_hi) (local.get $product0))
            (call $overflow)
        )

        ;; (a3b0 + a2b1 + a1b2 + a0b3) << 96
        ;; a3b0
        (local.set $product0 (i64.mul (i64.extend_i32_u (local.get $a3)) (i64.extend_i32_u (local.get $b0))))
        ;; a2b1
        (local.set $product1 (i64.mul (i64.extend_i32_u (local.get $a2)) (i64.extend_i32_u (local.get $b1))))
        ;; a1b2
        (local.set $product2 (i64.mul (i64.extend_i32_u (local.get $a1)) (i64.extend_i32_u (local.get $b2))))
        ;; a0b3
        (local.set $product3 (i64.mul (i64.extend_i32_u (local.get $a0)) (i64.extend_i32_u (local.get $b3))))
        ;; a3b0 + a2b1
        (local.set $product0 (i64.add (local.get $product0) (local.get $product1)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product1))
            (call $overflow)
        )
        ;; a3b0 + a2b1 + a1b2
        (local.set $product0 (i64.add (local.get $product0) (local.get $product2)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product2))
            (call $overflow)
        )
        ;; a3b0 + a2b1 + a1b2 + a0b3
        (local.set $product0 (i64.add (local.get $product0) (local.get $product3)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product3))
            (call $overflow)
        )
        ;; check for overflow in upper 32 bits of result
        (if (i64.ne (i64.shr_u (local.get $product0) (i64.const 32)) (i64.const 0))
            (call $overflow)
        )
        ;; result += (a3b0 + a2b1 + a1b2 + a0b3) << 96
        (local.set $product0 (i64.shl (local.get $product0) (i64.const 32)))
        (local.set $res_hi (i64.add (local.get $res_hi) (local.get $product0)))
        ;; check for carry
        (if (i64.lt_u (local.get $res_hi) (local.get $product0))
            (call $overflow)
        )

        ;; Return the result
        (return (local.get $res_hi) (local.get $res_lo))
    )

    (func $mul-int (param $a_hi i64) (param $a_lo i64) (param $b_hi i64) (param $b_lo i64) (result i64 i64)
        (local $res_hi i64)
        (local $res_lo i64)
        (local $sign_a i64)
        (local $sign_b i64)
        (local $expected_sign i64)

        ;; Shortcut if either a or b is zero (repeated here to avoid overflow check)
        (if (i32.or
                (i64.eqz (i64.or (local.get $a_hi) (local.get $a_lo)))
                (i64.eqz (i64.or (local.get $b_hi) (local.get $b_lo))))
            (return (i64.const 0) (i64.const 0))
        )

        (local.get $a_hi)
        (local.get $a_lo)
        (local.get $b_hi)
        (local.get $b_lo)
        (call $mul-int128)

        (local.set $res_lo)
        (local.set $res_hi)

        ;; Check for overflow into sign bit
        (local.set $sign_a (i64.shr_s (local.get $a_hi) (i64.const 63)))
        (local.set $sign_b (i64.shr_s (local.get $b_hi) (i64.const 63)))
        (local.set $expected_sign (i64.xor (local.get $sign_a) (local.get $sign_b)))
        (if (i64.ne (i64.shr_s (local.get $res_hi) (i64.const 63)) (local.get $expected_sign))
            (call $overflow)
        )

        ;; Return the result
        (return (local.get $res_hi) (local.get $res_lo))
    )

    (export "add-uint" (func $add-uint))
    (export "add-int" (func $add-int))
    (export "sub-uint" (func $sub-uint))
    (export "sub-int" (func $sub-int))
    (export "mul-uint" (func $mul-int128))
    (export "mul-int" (func $mul-int))
)