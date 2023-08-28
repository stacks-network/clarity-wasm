;; This module contains a standard library for Clarity, defining Clarity's
;; builtins, to be called from the generated Wasm code.
(module
    (type (;0;) (func (param i32)))
    (type (;1;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)))
    (type (;2;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64 i64 i64)))
    (type (;3;) (func (param i64 i64) (result i64 i64)))
    (type (;4;) (func (param i32 i32 i32) (result i32)))
    (type (;5;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)))

    ;; Functions imported for host interface
    ;; define_variable(var_id: i32, name: string (offset: i32, length: i32), initial_value: (offset: i32, length: i32))
    (import "clarity" "define_variable" (func $define_variable (param i32 i32 i32 i32 i32)))
    ;; get_variable(var_id: i32, return_val: (offset: i32, length: i32))
    (import "clarity" "get_variable" (func $get_variable (param i32 i32 i32)))
    ;; set_variable(var_id: i32, value: (offset: i32, length: i32))
    (import "clarity" "set_variable" (func $set_variable (param i32 i32 i32)))

    (global $stack-pointer (mut i32) (i32.const 0))
    (memory (export "memory") 10)

    ;; The error code is one of:
        ;; 0: overflow
        ;; 1: underflow
        ;; 2: divide by zero
        ;; 3: log of a number <= 0
    (func $runtime-error (type 0) (param $error-code i32)
        ;; TODO: Implement runtime error
        unreachable
    )

    ;; Copies a range of bytes from one location in memory to another. It is
    ;; assumed that the source and destination ranges do not overlap.
    ;; Returns the ending destination offset.
    ;; TODO: This can be optimized to use 32-bit load/stores if the source and
    ;; destination are both 32-bit aligned.
    (func $memcpy (type 4) (param $src_offset i32) (param $src_size i32) (param $dest_offset i32) (result i32)
        (local $end i32)
        (local $tmp i32)

        (local.set $end (i32.add (local.get $src_offset) (local.get $src_size)))
        (block $done
            (loop $loop
                ;; Check if we've copied all bytes
                (if (i32.eq (local.get $src_offset) (local.get $end))
                    (br $done)
                )

                ;; Load byte from source
                (local.set $tmp (i32.load8_u (local.get $src_offset)))

                ;; Store byte to destination
                (i32.store8 (local.get $dest_offset) (local.get $tmp))

                ;; Increment offsets
                (local.set $src_offset (i32.add (local.get $src_offset) (i32.const 1)))
                (local.set $dest_offset (i32.add (local.get $dest_offset) (i32.const 1)))

                ;; Continue loop
                (br $loop)
            )
        )
        (return (local.get $dest_offset))
    )


    ;; This function can be used to add either signed or unsigned integers
    (func $add-int128 (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
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
        (return (local.get $sum_lo) (local.get $sum_hi))
    )

    (func $add-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $sum_hi i64)
        (local $sum_lo i64)

        (local.get $a_lo)
        (local.get $a_hi)
        (local.get $b_lo)
        (local.get $b_hi)
        (call $add-int128)

        (local.set $sum_hi)
        (local.set $sum_lo)

        ;; Check for overflow and underflow
        (if (i64.eq (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $b_hi) (i64.const 63))) ;; if a and b have the same sign
            (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $sum_hi) (i64.const 63))) ;; and the result has a different sign
                (call $runtime-error (i32.const 0))
            )
        )

        ;; Return the result
        (return (local.get $sum_lo) (local.get $sum_hi))
    )

    (func $add-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $sum_hi i64)
        (local $sum_lo i64)

        (local.get $a_lo)
        (local.get $a_hi)
        (local.get $b_lo)
        (local.get $b_hi)
        (call $add-int128)

        (local.set $sum_hi)
        (local.set $sum_lo)

        ;; Check for overflow
        (if (i64.lt_u (local.get $sum_hi) (local.get $a_hi))
            (call $runtime-error (i32.const 0))
        )

        ;; Return the result
        (return (local.get $sum_lo) (local.get $sum_hi))
    )

    ;; This function can be used to subtract either signed or unsigned integers
    (func $sub-int128 (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
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
        (return (local.get $diff_lo) (local.get $diff_hi))
    )

    (func $sub-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $diff_hi i64)
        (local $diff_lo i64)

        (local.get $a_lo)
        (local.get $a_hi)
        (local.get $b_lo)
        (local.get $b_hi)
        (call $sub-int128)

        (local.set $diff_hi)
        (local.set $diff_lo)

        ;; Check for overflow and underflow
        (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $b_hi) (i64.const 63))) ;; if a and b have different signs
            (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $diff_hi) (i64.const 63))) ;; and the result has a different sign from a
                (call $runtime-error (i32.const 1))
            )
        )

        ;; Return the result
        (return (local.get $diff_lo) (local.get $diff_hi))
    )

    (func $sub-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $diff_hi i64)
        (local $diff_lo i64)

        (local.get $a_lo)
        (local.get $a_hi)
        (local.get $b_lo)
        (local.get $b_hi)
        (call $sub-int128)

        (local.set $diff_hi)
        (local.set $diff_lo)

        ;; Check for underflow
        (if (i64.gt_u (local.get $diff_hi) (local.get $a_hi))
            (call $runtime-error (i32.const 1))
        )

        ;; Return the result
        (return (local.get $diff_lo) (local.get $diff_hi))
    )

    (func $mul-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
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
        ;; We need to make sure these are 0 or report overflow if they are not
        (if (i32.or
                (i32.ne (i32.or (local.get $a3) (local.get $b1)) (i32.const 0))
                (i32.ne (i32.or (local.get $a2) (local.get $b2)) (i32.const 0))
                (i32.ne (i32.or (local.get $a1) (local.get $b3)) (i32.const 0))
                (i32.ne (i32.or (local.get $a3) (local.get $b2)) (i32.const 0))
                (i32.ne (i32.or (local.get $a2) (local.get $b3)) (i32.const 0))
                (i32.ne (i32.or (local.get $a3) (local.get $b3)) (i32.const 0))
            )
            (call $runtime-error (i32.const 0))
        )
        
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
            (call $runtime-error (i32.const 0))
        )
        (local.set $product0 (i64.add (local.get $product0) (local.get $product2)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product2))
            (call $runtime-error (i32.const 0))
        )
        ;; res_hi += (a2b0 + a1b1 + a0b2)
        (local.set $res_hi (i64.add (local.get $res_hi) (local.get $product0)))
        ;; check for carry
        (if (i64.lt_u (local.get $res_hi) (local.get $product0))
            (call $runtime-error (i32.const 0))
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
            (call $runtime-error (i32.const 0))
        )
        ;; a3b0 + a2b1 + a1b2
        (local.set $product0 (i64.add (local.get $product0) (local.get $product2)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product2))
            (call $runtime-error (i32.const 0))
        )
        ;; a3b0 + a2b1 + a1b2 + a0b3
        (local.set $product0 (i64.add (local.get $product0) (local.get $product3)))
        ;; check for carry
        (if (i64.lt_u (local.get $product0) (local.get $product3))
            (call $runtime-error (i32.const 0))
        )
        ;; check for overflow in upper 32 bits of result
        (if (i64.ne (i64.shr_u (local.get $product0) (i64.const 32)) (i64.const 0))
            (call $runtime-error (i32.const 0))
        )
        ;; result += (a3b0 + a2b1 + a1b2 + a0b3) << 96
        (local.set $product0 (i64.shl (local.get $product0) (i64.const 32)))
        (local.set $res_hi (i64.add (local.get $res_hi) (local.get $product0)))
        ;; check for carry
        (if (i64.lt_u (local.get $res_hi) (local.get $product0))
            (call $runtime-error (i32.const 0))
        )

        ;; Return the result
        (return (local.get $res_lo) (local.get $res_hi))
    )

    (func $mul-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
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

        (local.get $a_lo)
        (local.get $a_hi)
        (local.get $b_lo)
        (local.get $b_hi)
        (call $mul-uint)

        (local.set $res_hi)
        (local.set $res_lo)

        ;; Check for overflow into sign bit
        (local.set $sign_a (i64.shr_s (local.get $a_hi) (i64.const 63)))
        (local.set $sign_b (i64.shr_s (local.get $b_hi) (i64.const 63)))
        (local.set $expected_sign (i64.xor (local.get $sign_a) (local.get $sign_b)))
        (if (i64.ne (i64.shr_s (local.get $res_hi) (i64.const 63)) (local.get $expected_sign))
            (call $runtime-error (i32.const 0))
        )

        ;; Return the result
        (return (local.get $res_lo) (local.get $res_hi))
    )

    (func $div-int128 (type 2) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64 i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)
        (local $current_bit i64)

        ;; Check for division by 0
        (if (i64.eqz (i64.or (local.get $divisor_hi) (local.get $divisor_lo)))
            (call $runtime-error (i32.const 2))
        )

        ;; Long division algorithm
        ;; Initialize the quotient and remainder to 0
        (local.set $quotient_hi (i64.const 0))
        (local.set $quotient_lo (i64.const 0))
        (local.set $remainder_hi (i64.const 0))
        (local.set $remainder_lo (i64.const 0))
        ;; (local.set $remainder_hi (local.get $dividend_hi))
        ;; (local.set $remainder_lo (local.get $dividend_lo))

        ;; Use current_bit to loop over the bits of the dividend
        (local.set $current_bit (i64.const 127))

        (loop $div_loop
            ;; Shift the remainder left by one bit, 
            ;; filling the least significant bit with the next bit of the dividend
            (local.set $remainder_hi (i64.or
                (i64.shl (local.get $remainder_hi) (i64.const 1))
                (i64.shr_u (local.get $remainder_lo) (i64.const 63))))
            (local.set $remainder_lo (i64.or
                (i64.shl (local.get $remainder_lo) (i64.const 1))
                (i64.shr_u (local.get $dividend_hi) (i64.const 63))))

            ;; Shift the dividend left by one bit
            (local.set $dividend_hi (i64.or
                (i64.shl (local.get $dividend_hi) (i64.const 1))
                (i64.shr_u (local.get $dividend_lo) (i64.const 63))))
            (local.set $dividend_lo (i64.shl (local.get $dividend_lo) (i64.const 1)))

            ;; If the remainder is greater than or equal to the divisor,
            (if (i32.or (i64.gt_u (local.get $remainder_hi) (local.get $divisor_hi))
                        (i32.and (i64.eq (local.get $remainder_hi) (local.get $divisor_hi))
                                 (i64.ge_u (local.get $remainder_lo) (local.get $divisor_lo))))
                (then
                    ;; Subtract the divisor from the remainder
                    (call $sub-int128 (local.get $remainder_lo) (local.get $remainder_hi) (local.get $divisor_lo) (local.get $divisor_hi))
                    (local.set $remainder_hi)
                    (local.set $remainder_lo)

                    ;; and set the current bit of the quotient to 1
                    (if (i64.lt_u (local.get $current_bit) (i64.const 64))
                        (then
                            (local.set $quotient_lo (i64.or (local.get $quotient_lo)
                                (i64.shl (i64.const 1) (local.get $current_bit))))
                        )
                        (else
                            (local.set $quotient_hi (i64.or (local.get $quotient_hi)
                                (i64.shl (i64.const 1) (i64.sub (local.get $current_bit) (i64.const 64)))))
                        )
                    )
                )
            )

            ;; Decrement the current bit and loop until all bits have been processed
            (local.set $current_bit (i64.sub (local.get $current_bit) (i64.const 1)))
            (br_if $div_loop (i64.ge_s (local.get $current_bit) (i64.const 0)))
        )

        ;; Return the quotient and the remainder
        (return (local.get $quotient_lo) (local.get $quotient_hi) (local.get $remainder_lo) (local.get $remainder_hi))
    )

    (func $div-uint (type 1) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)

        (call $div-int128 (local.get $dividend_lo) (local.get $dividend_hi) (local.get $divisor_lo) (local.get $divisor_hi))
        (local.set $remainder_hi)
        (local.set $remainder_lo)
        (local.set $quotient_hi)
        (local.set $quotient_lo)

        (return (local.get $quotient_lo) (local.get $quotient_hi))
    )

    (func $div-int (type 1) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)
        (local $sign_dividend i64)
        (local $sign_divisor i64)
        (local $expected_sign i64)

        ;; Compute the expected sign of the result
        (local.set $sign_dividend (i64.shr_s (local.get $dividend_hi) (i64.const 63)))
        (local.set $sign_divisor (i64.shr_s (local.get $divisor_hi) (i64.const 63)))
        (local.set $expected_sign (i64.xor (local.get $sign_dividend) (local.get $sign_divisor)))

        ;; Perform the division using the absolute values of the operands
        (if (i32.wrap_i64 (local.get $sign_dividend))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $dividend_lo) (local.get $dividend_hi))
                (local.set $dividend_hi)
                (local.set $dividend_lo)
            )
        )
        (if (i32.wrap_i64 (local.get $sign_divisor))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $divisor_lo) (local.get $divisor_hi))
                (local.set $divisor_hi)
                (local.set $divisor_lo)
            )
        )

        (call $div-int128 (local.get $dividend_lo) (local.get $dividend_hi) (local.get $divisor_lo) (local.get $divisor_hi))
        (local.set $remainder_hi)
        (local.set $remainder_lo)
        (local.set $quotient_hi)
        (local.set $quotient_lo)

        ;; If the result should be negative, negate it
        (if (i32.wrap_i64 (local.get $expected_sign))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $quotient_lo) (local.get $quotient_hi))
                (local.set $quotient_hi)
                (local.set $quotient_lo)
            )
        )

        (return (local.get $quotient_lo) (local.get $quotient_hi))
    )

    (func $mod-uint (type 1) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)

        (call $div-int128 (local.get $dividend_lo) (local.get $dividend_hi) (local.get $divisor_lo) (local.get $divisor_hi))
        (local.set $remainder_hi)
        (local.set $remainder_lo)
        (local.set $quotient_hi)
        (local.set $quotient_lo)

        (return (local.get $remainder_lo) (local.get $remainder_hi))
    )

    (func $mod-int (type 1) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)
        (local $sign_dividend i64)
        (local $sign_divisor i64)
        (local $expected_sign i64)

        ;; Compute the expected sign of the result
        (local.set $sign_dividend (i64.shr_s (local.get $dividend_hi) (i64.const 63)))
        (local.set $sign_divisor (i64.shr_s (local.get $divisor_hi) (i64.const 63)))
        (local.set $expected_sign (i64.xor (local.get $sign_dividend) (local.get $sign_divisor)))

        ;; Perform the division using the absolute values of the operands
        (if (i32.wrap_i64 (local.get $sign_dividend))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $dividend_lo) (local.get $dividend_hi))
                (local.set $dividend_hi)
                (local.set $dividend_lo)
            )
        )
        (if (i32.wrap_i64 (local.get $sign_divisor))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $divisor_lo) (local.get $divisor_hi))
                (local.set $divisor_hi)
                (local.set $divisor_lo)
            )
        )

        (call $div-int128 (local.get $dividend_lo) (local.get $dividend_hi) (local.get $divisor_lo) (local.get $divisor_hi))
        (local.set $remainder_hi)
        (local.set $remainder_lo)
        (local.set $quotient_hi)
        (local.set $quotient_lo)

        ;; If the result should be negative, negate it
        (if (i32.wrap_i64 (local.get $sign_dividend))
            (then
                (call $sub-int128 (i64.const 0) (i64.const 0) (local.get $remainder_lo) (local.get $remainder_hi))
                (local.set $remainder_hi)
                (local.set $remainder_lo)
            )
        )

        (return (local.get $remainder_lo) (local.get $remainder_hi))
    )

    (func $lt-uint (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.lt_u (local.get $a_lo) (local.get $b_lo))
            (i64.lt_u (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $gt-uint (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.gt_u (local.get $a_lo) (local.get $b_lo))
            (i64.gt_u (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $le-uint (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.le_u (local.get $a_lo) (local.get $b_lo))
            (i64.le_u (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $ge-uint (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.ge_u (local.get $a_lo) (local.get $b_lo))
            (i64.ge_u (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $lt-int (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.lt_u (local.get $a_lo) (local.get $b_lo))
            (i64.lt_s (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $gt-int (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.gt_u (local.get $a_lo) (local.get $b_lo))
            (i64.gt_s (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $le-int (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.le_u (local.get $a_lo) (local.get $b_lo))
            (i64.le_s (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $ge-int (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (select
            (i64.ge_u (local.get $a_lo) (local.get $b_lo))
            (i64.ge_s (local.get $a_hi) (local.get $b_hi))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

    (func $log2 (param $lo i64) (param $hi i64) (result i64)
        (select
            (i64.xor (i64.clz (local.get $lo)) (i64.const 63))
            (i64.xor (i64.clz (local.get $hi)) (i64.const 127))
            (i64.eqz (local.get $hi))
        )
    )

    (func $log2-uint (type 3) (param $lo i64) (param $hi i64) (result i64 i64)
        (if (i64.eqz (i64.or (local.get $hi) (local.get $lo)))
            (call $runtime-error (i32.const 3)))
        (call $log2 (local.get $lo) (local.get $hi))
        (i64.const 0)
    )

    (func $log2-int (type 3) (param $lo i64) (param $hi i64) (result i64 i64)
        (if (call $le-int (local.get $lo) (local.get $hi) (i64.const 0) (i64.const 0))
            (call $runtime-error (i32.const 3)))
        (call $log2 (local.get $lo) (local.get $hi))
        (i64.const 0)
    )

    (export "memcpy" (func $memcpy))
    (export "add-uint" (func $add-uint))
    (export "add-int" (func $add-int))
    (export "sub-uint" (func $sub-uint))
    (export "sub-int" (func $sub-int))
    (export "mul-uint" (func $mul-uint))
    (export "mul-int" (func $mul-int))
    (export "div-uint" (func $div-uint))
    (export "div-int" (func $div-int))
    (export "mod-uint" (func $mod-uint))
    (export "mod-int" (func $mod-int))
    (export "lt-uint" (func $lt-uint))
    (export "gt-uint" (func $gt-uint))
    (export "le-uint" (func $le-uint))
    (export "ge-uint" (func $ge-uint))
    (export "lt-int" (func $lt-int))
    (export "gt-int" (func $gt-int))
    (export "le-int" (func $le-int))
    (export "ge-int" (func $ge-int))
    (export "log2-uint" (func $log2-uint))
    (export "log2-int" (func $log2-int))
)
