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
    (import "clarity" "define_function" (func $define_function (param $kind i32)
                                                               (param $name_offset i32)
                                                               (param $name_length i32)))
    (import "clarity" "define_variable" (func $define_variable (param $name_offset i32)
                                                               (param $name_length i32)
                                                               (param $initial_value_offset i32)
                                                               (param $initial_value_length i32)))
    (import "clarity" "define_ft" (func $define_ft (param $name_offset i32)
                                                   (param $name_length i32)
                                                   (param $supply_indicator i32)
                                                   (param $supply_lo i64)
                                                   (param $supply_hi i64)))
    (import "clarity" "define_nft" (func $define_nft (param $name_offset i32)
                                                     (param $name_length i32)))
    (import "clarity" "define_map" (func $define_map (param $name_offset i32)
                                                     (param $name_length i32)))

    (import "clarity" "get_variable" (func $get_variable (param $name_offset i32)
                                                         (param $name_length i32)
                                                         (param $return_offset i32)
                                                         (param $return_length i32)))
    (import "clarity" "set_variable" (func $set_variable (param $name_offset i32)
                                                         (param $name_length i32)
                                                         (param $value_offset i32)
                                                         (param $value_length i32)))
    (import "clarity" "enter_as_contract" (func $enter_as_contract))
    (import "clarity" "exit_as_contract" (func $exit_as_contract))
    (import "clarity" "stx_get_balance" (func $stx_get_balance (param $principal_offset i32)
                                                               (param $principal_length i32)
                                                               (result i64 i64)))
    (import "clarity" "stx_account" (func $stx_account (param $principal_offset i32)
                                                       (param $principal_length i32)
                                                       (result i64 i64 i64 i64 i64 i64)))
    (import "clarity" "stx_burn" (func $stx_burn (param $amount_lo i64)
                                                 (param $amount_hi i64)
                                                 (param $principal_offset i32)
                                                 (param $principal_length i32)
                                                 (result i32 i32 i64 i64)))
    (import "clarity" "stx_transfer" (func $stx_transfer (param $amount_lo i64)
                                                         (param $amount_hi i64)
                                                         (param $sender_offset i32)
                                                         (param $sender_length i32)
                                                         (param $recipient_offset i32)
                                                         (param $recipient_length i32)
                                                         (param $memo_offset i32)
                                                         (param $memo_length i32)
                                                         (result i32 i32 i64 i64)))

    (import "clarity" "ft_get_supply" (func $ft_get_supply (param $name_offset i32)
                                                           (param $name_length i32)
                                                           (result i64 i64)))
    (import "clarity" "ft_get_balance" (func $ft_get_balance (param $name_offset i32)
                                                             (param $name_length i32)
                                                             (param $owner_offset i32)
                                                             (param $owner_length i32)
                                                             (result i64 i64)))
    (import "clarity" "ft_burn" (func $ft_burn (param $name_offset i32)
                                               (param $name_length i32)
                                               (param $amount_lo i64)
                                               (param $amount_hi i64)
                                               (param $sender_offset i32)
                                               (param $sender_length i32)
                                               (result i32 i32 i64 i64)))
    (import "clarity" "ft_mint" (func $ft_mint (param $name_offset i32)
                                               (param $name_length i32)
                                               (param $amount_lo i64)
                                               (param $amount_hi i64)
                                               (param $sender_offset i32)
                                               (param $sender_length i32)
                                               (result i32 i32 i64 i64)))
    (import "clarity" "ft_transfer" (func $ft_transfer (param $name_offset i32)
                                                       (param $name_length i32)
                                                       (param $amount_lo i64)
                                                       (param $amount_hi i64)
                                                       (param $sender_offset i32)
                                                       (param $sender_length i32)
                                                       (param $recipient_offset i32)
                                                       (param $recipient_length i32)
                                                       (result i32 i32 i64 i64)))
    (import "clarity" "nft_get_owner" (func $nft_get_owner (param $name_offset i32)
                                                           (param $name_length i32)
                                                           (param $asset_offset i32)
                                                           (param $asset_length i32)
                                                           (param $return_offset i32)
                                                           (param $return_length i32)
                                                           (result i32 i32 i32)))
    (import "clarity" "nft_burn" (func $nft_burn (param $name_offset i32)
                                                 (param $name_length i32)
                                                 (param $asset_offset i32)
                                                 (param $asset_length i32)
                                                 (param $sender_offset i32)
                                                 (param $sender_length i32)
                                                 (result i32 i32 i64 i64)))
    (import "clarity" "nft_mint" (func $nft_mint (param $name_offset i32)
                                                 (param $name_length i32)
                                                 (param $asset_offset i32)
                                                 (param $asset_length i32)
                                                 (param $recipient_offset i32)
                                                 (param $recipient_length i32)
                                                 (result i32 i32 i64 i64)))
    (import "clarity" "nft_transfer" (func $nft_transfer (param $name_offset i32)
                                                         (param $name_length i32)
                                                         (param $asset_offset i32)
                                                         (param $asset_length i32)
                                                         (param $sender_offset i32)
                                                         (param $sender_length i32)
                                                         (param $recipient_offset i32)
                                                         (param $recipient_length i32)
                                                         (result i32 i32 i64 i64)))
    (import "clarity" "map_get" (func $map_get (param $name_offset i32)
                                               (param $name_length i32)
                                               (param $key_offset i32)
                                               (param $key_length i32)
                                               (param $return_offset i32)
                                               (param $return_length i32)))
    (import "clarity" "map_set" (func $map_set (param $name_offset i32)
                                               (param $name_length i32)
                                               (param $key_offset i32)
                                               (param $key_length i32)
                                               (param $value_offset i32)
                                               (param $value_length i32)
                                               (result i32)))
    (import "clarity" "map_insert" (func $map_insert (param $name_offset i32)
                                                     (param $name_length i32)
                                                     (param $key_offset i32)
                                                     (param $key_length i32)
                                                     (param $value_offset i32)
                                                     (param $value_length i32)
                                                     (result i32)))
    (import "clarity" "map_delete" (func $map_delete (param $name_offset i32)
                                                     (param $name_length i32)
                                                     (param $key_offset i32)
                                                     (param $key_length i32)
                                                     (result i32)))
    (import "clarity" "get_block_info" (func $get_block_info (param $name_offset i32)
                                                             (param $name_length i32)
                                                             (param $height_lo i64)
                                                             (param $height_hi i64)
                                                             (param $return_offset i32)
                                                             (param $return_length i32)))
    (import "clarity" "static_contract_call" (func $static_contract_call (param $contract_id_offset i32)
                                                                         (param $contract_id_length i32)
                                                                         (param $function_name_offset i32)
                                                                         (param $function_name_length i32)
                                                                         (param $arguments_offset i32)
                                                                         (param $arguments_length i32)
                                                                         (param $return_offset i32)
                                                                         (param $return_length i32)))

    (import "clarity" "tx_sender" (func $tx_sender (param $return_offset i32)
                                                   (param $return_length i32)
                                                   (result i32 i32)))
    (import "clarity" "contract_caller" (func $contract_caller (param $return_offset i32)
                                                               (param $return_length i32)
                                                               (result i32 i32)))
    (import "clarity" "tx_sponsor" (func $tx_sponsor (param $return_offset i32)
                                                     (param $return_length i32)
                                                     (result i32 i32 i32)))
    (import "clarity" "block_height" (func $block_height (result i64 i64)))
    (import "clarity" "burn_block_height" (func $burn_block_height (result i64 i64)))
    (import "clarity" "stx_liquid_supply" (func $stx_liquid_supply (result i64 i64)))
    ;; TODO: these three funcs below could be hard-coded at compile-time.
    (import "clarity" "is_in_regtest" (func $is_in_regtest (result i32)))
    (import "clarity" "is_in_mainnet" (func $is_in_mainnet (result i32)))
    (import "clarity" "chain_id" (func $chain_id (result i64 i64)))

    (global $stack-pointer (mut i32) (i32.const 0))
    (export "stack-pointer" (global $stack-pointer))
    (memory (export "memory") 10)

    ;; The error code is one of:
        ;; 0: overflow
        ;; 1: underflow
        ;; 2: divide by zero
        ;; 3: log of a number <= 0
        ;; 4: expected a non-negative number
        ;; 5: panic
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
                    (then (br $done))
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
        ;; Add the lower 64 bits
        (local.tee $b_lo (i64.add (local.get $a_lo) (local.get $b_lo))) ;; $b_lo now contains the result lower bits

        ;; Add the upper 64 bits, accounting for any carry from the lower bits
        (i64.add
            (i64.extend_i32_u (i64.gt_u (local.get $a_lo) (local.get $b_lo)))   ;; carry
            (i64.add (local.get $a_hi) (local.get $b_hi)))                      ;; upper 64 bits
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
            (then
                (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $sum_hi) (i64.const 63))) ;; and the result has a different sign
                    (then (call $runtime-error (i32.const 0)))
                )
            )
        )

        ;; Return the result
        (return (local.get $sum_lo) (local.get $sum_hi))
    )

    (func $add-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (call $add-int128 (local.get $a_lo) (local.get $a_hi) (local.get $b_lo) (local.get $b_hi))
        (local.set $a_hi) ;; storing the result in place of first operand
        (local.set $a_lo)

        ;; overflow condition: sum (a) < operand (b)
        (if (call $lt-uint (local.get $a_lo) (local.get $a_hi) (local.get $b_lo) (local.get $b_hi))
            (then (call $runtime-error (i32.const 0)))
        )

        (local.get $a_lo) (local.get $a_hi)
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
            (then
                (if (i64.ne (i64.shr_s (local.get $a_hi) (i64.const 63)) (i64.shr_s (local.get $diff_hi) (i64.const 63))) ;; and the result has a different sign from a
                    (then (call $runtime-error (i32.const 1)))
                )
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
            (then (call $runtime-error (i32.const 1)))
        )

        ;; Return the result
        (return (local.get $diff_lo) (local.get $diff_hi))
    )

    (func $mul-int128 (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
    ;; Adaptation of Hacker's Delight, chapter 8
    ;; u1 <- $a_lo & 0xffffffff; v1 <- $b_lo & 0xffffffff
    ;; u2 <- $a_lo >> 32; v2 <- $b_lo >> 32
    ;; t1 <- v1 * u1
    ;; t2 <- (u2 * v1) + (t1 >> 32)
    ;; t3 <- (u1 * v2) + (t2 & 0xffffffff)
    ;; $res_lo <- (t3 << 32) | (t1 & 0xffffffff)
    ;; $res_hi <- ($a_lo * b_hi) + ($a_hi * b_lo) + (v2 * u2) + (t2 >> 32) + (t3 >> 32)
        (local $u2 i64) (local $v2 i64) (local $t1 i64) (local $t2 i64) (local $t3 i64)

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;; res_lo ;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        (local.tee $t1
            (i64.mul
                ;; $v2 contains u1 temporarily
                (local.tee $v2 (i64.and (local.get $a_lo) (i64.const 0xffffffff)))
                ;; $u2 contains v1 temporarily
                (local.tee $u2 (i64.and (local.get $b_lo) (i64.const 0xffffffff)))
            )
        )
        (i64.shr_u (i64.const 32))              ;; (t1 >> 32)
        (i64.mul                                ;; (u2 * v1)
            (local.get $u2) ;; contains v1 at that point
            (local.tee $u2 (i64.shr_u (local.get $a_lo) (i64.const 32)))
        )
        (local.tee $t2 (i64.add))               ;; (u2 * v1) + (t1 >> 32)
        (i64.and (i64.const 0xffffffff))        ;; (t2 & 0xffffffff)
        (i64.mul                                ;; (u1 * v2)
            (local.get $v2) ;; contains u1 at that point
            (local.tee $v2 (i64.shr_u (local.get $b_lo) (i64.const 32)))
        )
        (local.tee $t3 (i64.add))               ;; (u1 * v2) + (t2 & 0xffffffff)
        (i64.shl (i64.const 32))                ;; (t3 << 32)
        (i64.and (local.get $t1) (i64.const 0xffffffff))
        i64.or                                  ;; (t2 << 32) | (t1 & 0xffffffff)

        ;;;;;;;;;;;;;;;;;;;;;;;;;;;; res_hi ;;;;;;;;;;;;;;;;;;;;;;;;;;;;
        (i64.mul (local.get $a_lo) (local.get $b_hi))
        (i64.add (i64.mul (local.get $a_hi) (local.get $b_lo)))
        (i64.add (i64.mul (local.get $v2) (local.get $u2)))
        (i64.add (i64.shr_u (local.get $t2) (i64.const 32)))
        (i64.add (i64.shr_u (local.get $t3) (i64.const 32)))
    )

    (func $mul-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $tmp i32)

        (local.set $tmp  ;; tmp contains the sum of number of leading zeros of arguments
            (i32.add
                (call $clz-int128 (local.get $a_lo) (local.get $a_hi))
                (call $clz-int128 (local.get $b_lo) (local.get $b_hi))
            )
        )

        (if (i32.ge_u (local.get $tmp) (i32.const 128))
            (then
                ;; product cannot overflow if the sum of leading zeros is >= 128
                (return (call $mul-int128 (local.get $a_lo) (local.get $a_hi) (local.get $b_lo) (local.get $b_hi)))
            )
        )

        (if (i32.le_u (local.get $tmp) (i32.const 126))
            (then
                ;; product will overflow if the sum of leading zeros is <= 126
                (call $runtime-error (i32.const 0))
            )
        )

        ;; Other case might overflow. We compute (a * b/2) and check if result > 2**127
        ;;    -> if yes, overflow
        ;;    -> if not, we double the product, and add a one more time if b was odd

        ;; tmp is 1 if b was odd else 0
        (local.set $tmp (i32.wrap_i64 (i64.and (local.get $b_lo) (i64.const 1))))

        ;; b / 2
        (i64.or
            (i64.shl (local.get $b_hi) (i64.const 63))
            (i64.shr_u (local.get $b_lo) (i64.const 1))
        )
        (i64.shr_u (local.get $b_hi) (i64.const 1))

        ;; a * b/2
        (call $mul-int128 (local.get $a_lo) (local.get $a_hi))
        ;; b contains the result from now on
        (local.set $b_hi)
        (local.set $b_lo)

        ;; if result/2 > 2**127 overflow
        (if (i64.lt_s (local.get $b_hi) (i64.const 0))
            (then (call $runtime-error (i32.const 0)))
        )

        ;; res *= 2, meaning res <<= 1
        (local.set $b_hi
            (i64.or
                (i64.shl (local.get $b_hi) (i64.const 1))
                (i64.shr_u (local.get $b_lo) (i64.const 63))
            )
        )
        (local.set $b_lo (i64.shl (local.get $b_lo) (i64.const 1)))

        ;; if b is odd ($tmp), we add a
        (if (local.get $tmp)
            (then
                (call $add-uint (local.get $b_lo) (local.get $b_hi) (local.get $a_lo) (local.get $a_hi))
                (local.set $b_hi)
                (local.set $b_lo)
            )
        )
        (local.get $b_lo) (local.get $b_hi)
    )


    (func $mul-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $sign i32)

        ;; this is a shortcut for a multiplication by 1.
        ;; also, it prevents us from dealing with the infamous abs(i128::MIN), and
        ;; the only operation that would work on that number would be (i128::MIN * 1)
        (if (i64.eqz (i64.or (local.get $a_hi) (i64.xor (local.get $a_lo) (i64.const 1))))
            (then (return (local.get $b_lo) (local.get $b_hi)))
        )
        (if (i64.eqz (i64.or (local.get $b_hi) (i64.xor (local.get $b_lo) (i64.const 1))))
            (then (return (local.get $a_lo) (local.get $a_hi)))
        )

        ;; take the absolute value of the operands, and compute the expected sign in 3 steps:
        ;; 1. Absolute value of a
        ;; NOTE: the absolute value algorithm was generated from
        ;;       `fn abs(a: i128) -> i128 { a.abs() }`
        (select
            (i64.sub (i64.const 0) (local.get $a_lo))
            (local.get $a_lo)
            (local.tee $sign (i64.lt_s (local.get $a_hi) (i64.const 0))) ;; sign is the sign bit of a
        )
        (select
            (i64.sub (i64.const 0) (i64.add (local.get $a_hi) (i64.extend_i32_u (i64.ne (local.get $a_lo) (i64.const 0)))))
            (local.get $a_hi)
            (local.get $sign)
        )
        ;; 2. Absolute value of b
        (select
            (i64.sub (i64.const 0) (local.get $b_lo))
            (local.get $b_lo)
            (local.tee $sign (i64.lt_s (local.get $b_hi) (i64.const 0))) ;; sign is sign bit of b now
        )
        (select
            (i64.sub (i64.const 0) (i64.add (local.get $b_hi) (i64.extend_i32_u (i64.ne (local.get $b_lo) (i64.const 0)))))
            (local.get $b_hi)
            (local.get $sign)
        )
        ;; 3. Compute expected sign
        (local.set $sign
            (i32.xor
                (i64.lt_s (local.get $a_hi) (i64.const 0)) ;; sign of a
                (local.get $sign) ;; sign is sign of b
            )
        )

        (call $mul-uint)
        (local.set $a_hi)
        (local.set $a_lo)

        ;; Sign bit should be 0, otherwise there is an overflow
        (if (i64.lt_s (local.get $a_hi) (i64.const 0))
            (then (call $runtime-error (i32.const 0)))
        )

        ;; Return the result and adapt with sign bit
        (select
            (i64.sub (i64.const 0) (local.get $a_lo))
            (local.get $a_lo)
            (local.get $sign)
        )
        (select
            (i64.sub (i64.const 0) (i64.add (local.get $a_hi) (i64.extend_i32_u (i64.ne (local.get $a_lo) (i64.const 0)))))
            (local.get $a_hi)
            (local.get $sign)
        )
    )

    (func $div-int128 (type 2) (param $dividend_lo i64) (param $dividend_hi i64) (param $divisor_lo i64) (param $divisor_hi i64) (result i64 i64 i64 i64)
        (local $quotient_hi i64)
        (local $quotient_lo i64)
        (local $remainder_hi i64)
        (local $remainder_lo i64)
        (local $current_bit i64)

        ;; Check for division by 0
        (if (i64.eqz (i64.or (local.get $divisor_hi) (local.get $divisor_lo)))
            (then (call $runtime-error (i32.const 2)))
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
            (then (call $runtime-error (i32.const 3)))
        )
        (call $log2 (local.get $lo) (local.get $hi))
        (i64.const 0)
    )

    (func $log2-int (type 3) (param $lo i64) (param $hi i64) (result i64 i64)
        (if (call $le-int (local.get $lo) (local.get $hi) (i64.const 0) (i64.const 0))
            (then (call $runtime-error (i32.const 3)))
        )
        (call $log2 (local.get $lo) (local.get $hi))
        (i64.const 0)
    )

    (func $sqrti-uint (type 3) (param $lo i64) (param $hi i64) (result i64 i64)
        ;; https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Binary_numeral_system_(base_2)
        (local $d_lo i64) (local $d_hi i64)
        (local $c_lo i64) (local $c_hi i64)
        (local $tmp_lo i64) (local $tmp_hi i64)

        (if (i64.eqz (i64.or (local.get $lo) (local.get $hi)))
            (then (return (i64.const 0) (i64.const 0)))
        )

        (local.set $c_lo (i64.const 0))
        (local.set $c_hi (i64.const 0))

        ;; computing d
        (if (i64.eqz (local.get $hi))
            (then
                ;; since we know $d_hi will be 0, we can use it as tmp value during computation
                (local.set $d_hi (i64.const 0x4000000000000000))
                (loop $loop_lo
                    (local.set $d_hi
                        (i64.shr_u
                            (local.tee $d_lo (local.get $d_hi))
                            (i64.const 2)
                        )
                    )
                    (br_if $loop_lo (i64.lt_u (local.get $lo) (local.get $d_lo)))
                )
                (local.set $d_hi (i64.const 0))
            )
            (else
                ;; since we know $d_lo will be 0, we can use it as tmp value during computation
                (local.set $d_lo (i64.const 0x4000000000000000))
                (loop $loop_hi
                    (local.set $d_lo
                        (i64.shr_u
                            (local.tee $d_hi (local.get $d_lo))
                            (i64.const 2)
                        )
                    )
                    (br_if $loop_hi (i64.lt_u (local.get $hi) (local.get $d_hi)))
                )
                (local.set $d_lo (i64.const 0))
            )
        )

        (loop $loop_res
            ;; tmp = c + d
            (call $add-int128 (local.get $c_lo) (local.get $c_hi) (local.get $d_lo) (local.get $d_hi))
            (local.set $tmp_hi)
            (local.set $tmp_lo)

            ;; c = c >> 1
            (local.set $c_lo
                (i64.or
                    (i64.shl (local.get $c_hi) (i64.const 63))
                    (i64.shr_u (local.get $c_lo) (i64.const 1))
                )
            )
            (local.set $c_hi (i64.shr_u (local.get $c_hi) (i64.const 1)))

            ;; if n >= tmp
            (if (call $ge-uint (local.get $lo) (local.get $hi) (local.get $tmp_lo) (local.get $tmp_hi))
                (then
                    ;; n -= tmp
                    (call $sub-int128 (local.get $lo) (local.get $hi) (local.get $tmp_lo) (local.get $tmp_hi))
                    (local.set $hi)
                    (local.set $lo)

                    ;; c += d
                    (call $add-int128 (local.get $c_lo) (local.get $c_hi) (local.get $d_lo) (local.get $d_hi))
                    (local.set $c_hi)
                    (local.set $c_lo)
                )
            )

            ;; d = d >> 2
            (local.set $d_lo
                (i64.or
                    (i64.shl (local.get $d_hi) (i64.const 62))
                    (i64.shr_u (local.get $d_lo) (i64.const 2))
                )
            )
            (local.set $d_hi (i64.shr_u (local.get $d_hi) (i64.const 2)))

            ;; branch if (d != 0)
            (br_if $loop_res
                (i64.ne (i64.or (local.get $d_lo) (local.get $d_hi)) (i64.const 0))
            )
        )

        (local.get $c_lo) (local.get $c_hi)
    )

    (func $sqrti-int (type 3) (param $lo i64) (param $hi i64) (result i64 i64)
        (if (i64.lt_s (local.get $hi) (i64.const 0))
            (then (call $runtime-error (i32.const 4)))
        )
        (call $sqrti-uint (local.get $lo) (local.get $hi))
    )

    (func $bit-and (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (i64.and (local.get $a_lo) (local.get $b_lo))
        (i64.and (local.get $a_hi) (local.get $b_hi))
    )

    (func $bit-not (type 3) (param $a_lo i64) (param $a_hi i64) (result i64 i64)
          ;; wasm does not have bitwise negation, but xoring with -1 is equivalent
          (i64.xor (local.get $a_lo) (i64.const -1))
          (i64.xor (local.get $a_hi) (i64.const -1))
    )

    (func $bit-or (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
          (i64.or (local.get $a_lo) (local.get $b_lo))
          (i64.or (local.get $a_hi) (local.get $b_hi))
    )

    (func $bit-xor (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
          (i64.xor (local.get $a_lo) (local.get $b_lo))
          (i64.xor (local.get $a_hi) (local.get $b_hi))
    )

    (func $bit-shift-left (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
          ;; only b_lo is useful here since we will take the reminder by 128
          ;; n % 128 == n & 127 == n & 0x7f
          (local.set $b_lo (i64.and (local.get $b_lo) (i64.const 0x7f)))
          ;; Two cases when we shift:
          ;; (1) we shift by a 0 <= shift < 64: we have to split the lower bits into the carried bits and the rest, then we shift
          ;;     the rest, we shift the higher part and we add the carry to the higher part.
          ;; (2) we shift by a 64 <= shift < 128: lower bits are automatically 0, and higher bits are the lower bits shifted by (shift - 64).
          (if (result i64 i64) (i64.lt_u (local.get $b_lo) (i64.const 64))
              (then ;; (1)
               (local.set $b_hi ;; using $b_hi for storing overflow bits
                          (select ;; that's a hack to workaround wasm shift by 64 has no effect
                           (i64.const 0)
                           (i64.shr_u (local.get $a_lo) (i64.sub (i64.const 64) (local.get $b_lo)))
                           (i64.eqz (local.get $b_lo))))
               (i64.shl (local.get $a_lo) (local.get $b_lo)) ;; lower_bits <<= shift
               (i64.or (i64.shl (local.get $a_hi) (local.get $b_lo))
                       (local.get $b_hi))) ;; higher_bits = (higher_bits << shift) | carry
              (else ;; (2)
               (i64.const 0)
               (i64.shl (local.get $a_lo) (i64.sub (local.get $b_lo) (i64.const 64))))))

    (func $bit-shift-right-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
		  ;; This is just an inverted version of shift-left, see above
          (local.set $b_lo (i64.and (local.get $b_lo) (i64.const 0x7f)))
          (if (result i64 i64) (i64.lt_u (local.get $b_lo) (i64.const 64))
              (then
               (local.set $b_hi
                          (select
                           (i64.const 0)
                           (i64.shl (local.get $a_hi)
                                    (i64.sub (i64.const 64)
                                             (local.get $b_lo)))
                           (i64.eqz (local.get $b_lo))))
               (i64.or (i64.shr_u (local.get $a_lo)
                                  (local.get $b_lo))
                       (local.get $b_hi))
               (i64.shr_u (local.get $a_hi)
                          (local.get $b_lo)))
              (else
               (i64.shr_u (local.get $a_hi)
                          (i64.sub (local.get $b_lo)
                                   (i64.const 64)))
               (i64.const 0))))

    (func $bit-shift-right-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
		  ;; This is just shift-right but taking into account the sign (using shr_s when shifting the high bits)
          (local.set $b_lo (i64.and (local.get $b_lo) (i64.const 0x7f)))
          (if (result i64 i64) (i64.lt_u (local.get $b_lo) (i64.const 64))
              (then
               (local.set $b_hi
                          (select
                           (i64.const 0)
                           (i64.shl (local.get $a_hi) (i64.sub (i64.const 64) (local.get $b_lo)))
                           (i64.eqz (local.get $b_lo))))
               (i64.or (i64.shr_u (local.get $a_lo) (local.get $b_lo))
                       (local.get $b_hi))
               (i64.shr_s (local.get $a_hi) (local.get $b_lo)))
              (else
               (i64.shr_s (local.get $a_hi) (i64.sub (local.get $b_lo) (i64.const 64)))
               ;; this keeps the sign from changing
               (i64.shr_s (local.get $a_hi) (i64.const 63)))))

    (func $clz-int128 (param $a_lo i64) (param $a_hi i64) (result i32)
        (i32.wrap_i64
            (select
                (i64.add (i64.const 64) (i64.clz (local.get $a_lo)))
                (i64.clz (local.get $a_hi))
                (i64.eqz (local.get $a_hi))
            )
        )
    )

    (func $pow-inner (param $a_lo i64) (param $a_hi i64) (param $b i32) (result i64 i64)
        ;; examples of this algo:
        ;; 3 ^ 5 => (3 ^ 4) * 3 => (9 ^ 2) * 3 => (81 ^ 1) * 3 => 243
        ;; 4 ^ 6 => (16 ^ 3) * 1 => (16 ^ 2) * 16 => (256 ^ 1) * 16 => 4096
        (local $carry_lo i64) (local $carry_hi i64)
        (local.set $carry_lo (i64.const 1))
        (local.set $carry_hi (i64.const 0))
        (loop
            (if (i32.eqz (i32.and (local.get $b) (i32.const 1)))
                (then
                    (local.set $b (i32.shr_u (local.get $b) (i32.const 1)))
                    (call $mul-uint (local.get $a_lo) (local.get $a_hi) (local.get $a_lo) (local.get $a_hi))
                    (local.set $a_hi)
                    (local.set $a_lo)
                )
                (else
                    (local.set $b (i32.xor (local.get $b) (i32.const 1)))
                    (call $mul-uint (local.get $a_lo) (local.get $a_hi) (local.get $carry_lo) (local.get $carry_hi))
                    (local.set $carry_hi)
                    (local.set $carry_lo)
                )
            )
            (br_if 0 (i32.gt_u (local.get $b) (i32.const 1)))
        )
        (call $mul-uint (local.get $a_lo) (local.get $a_hi) (local.get $carry_lo) (local.get $carry_hi))
    )

    (func $pow-uint (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        ;; (a == 0 && b == 0 => 1) & (b == 0 => 1) ==> (b == 0 => 1)
        (if (i64.eqz (i64.or (local.get $b_lo) (local.get $b_hi)))
            (then (return (i64.const 1) (i64.const 0)))
        )
        ;; (a == 0 => 0) & (a == 1 => 1) ==> (a < 2 => a)
        ;; also, (b == 1 => a)
        (if (i32.or
                (i32.and (i64.lt_u (local.get $a_lo) (i64.const 2)) (i64.eqz (local.get $a_hi)))
                (i64.eqz (i64.or (i64.xor (local.get $b_lo) (i64.const 1)) (local.get $b_hi)))
            )
            (then (return (local.get $a_lo) (local.get $a_hi)))
        )
        ;; if b > 127 -> runtime error: overflow (since the biggest b that doesn't
        ;; overflow is in 2^127)
        (if (i32.or
                (i64.gt_u (local.get $b_lo) (i64.const 127))
                (i64.ne (local.get $b_hi) (i64.const 0))
            )
            (then (call $runtime-error (i32.const 0)))
        )

        ;; shortcut if a == 2
        (if (i64.eqz (i64.or (i64.xor (local.get $a_lo) (i64.const 2)) (local.get $a_hi)))
            (then
                (return
                    (select
                        (i64.shl (i64.const 1) (local.get $b_lo))
                        (i64.const 0)
                        (i64.lt_u (local.get $b_lo) (i64.const 64))
                    )
                    (select
                        (i64.const 0)
                        (i64.shl (i64.const 1) (i64.sub (local.get $b_lo) (i64.const 64)))
                        (i64.lt_u (local.get $b_lo) (i64.const 64))
                    )
                )
            )
        )

        (call $pow-inner (local.get $a_lo) (local.get $a_hi) (i32.wrap_i64 (local.get $b_lo)))
    )

    (func $pow-int (type 1) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)
        (local $negative i32)
        ;; (a == 0 && b == 0 => 1) & (b == 0 => 1) ==> (b == 0 => 1)
        (if (i64.eqz (i64.or (local.get $b_lo) (local.get $b_hi)))
            (then (return (i64.const 1) (i64.const 0)))
        )


        ;; (a == 0 => 0) & (a == 1 => 1) ==> (a < 2 => a)
        ;; also, (b == 1 => a)
        (if (i32.or
                (i32.and (i64.lt_u (local.get $a_lo) (i64.const 2)) (i64.eqz (local.get $a_hi)))
                (i64.eqz (i64.or (i64.xor (local.get $b_lo) (i64.const 1)) (local.get $b_hi)))
            )
            (then (return (local.get $a_lo) (local.get $a_hi)))
        )

        ;; otherwise, if b < 0 => runtime error
        (if (i64.lt_s (local.get $b_hi) (i64.const 0))
            (then (call $runtime-error (i32.const 4)))
        )

        ;; if b > (a >= 0 ? 126 : 127) -> runtime error: overflow (since the biggest b that doesn't
        ;; overflow is in 2^126 and -2^127, and this is an edge case)
        (if (i64.gt_u
                (local.get $b_lo)
                (i64.add (i64.const 126) (i64.extend_i32_u (i64.lt_s (local.get $a_hi) (i64.const 0))))
            )
            (then (call $runtime-error (i32.const 0)))
        )

        ;; shortcut if a == 2
        (if (i64.eqz (i64.or (i64.xor (local.get $a_lo) (i64.const 2)) (local.get $a_hi)))
            (then
                (return
                    (select
                        (i64.shl (i64.const 1) (local.get $b_lo))
                        (i64.const 0)
                        (local.tee $negative (i64.lt_u (local.get $b_lo) (i64.const 64)))
                    )
                    (select
                        (i64.const 0)
                        (i64.shl (i64.const 1) (i64.sub (local.get $b_lo) (i64.const 64)))
                        (local.get $negative)
                    )
                )
            )
        )

        ;; shortcut if a == -2 (handles edge case -2^127)
        (if (i32.and (i64.eq (local.get $a_lo) (i64.const -2)) (i64.eq (local.get $a_hi) (i64.const -1)))
            (then
                (local.set $a_lo (select (i64.const -1) (i64.const 1) (local.tee $negative (i32.wrap_i64 (i64.and (local.get $b_lo) (i64.const 1))))))
                (local.set $a_hi (select (i64.const -1) (i64.const 0) (local.get $negative)))
                (return
                    (select
                        (i64.shl (local.get $a_lo) (local.get $b_lo))
                        (i64.const 0)
                        (local.tee $negative (i64.lt_u (local.get $b_lo) (i64.const 64)))
                    )
                    (select
                        (local.get $a_hi)
                        (i64.shl (local.get $a_lo) (i64.sub (local.get $b_lo) (i64.const 64)))
                        (local.get $negative)
                    )
                )
            )
        )

        ;; $pow-inner arguments: abs(a) and $b_lo
        ;; no need to care of i128::MIN, at this point it will overflow
        ;; abs($a_lo)
        (select
            (i64.sub (i64.const 0) (local.get $a_lo))
            (local.get $a_lo)
            (local.tee $negative (i64.lt_s (local.get $a_hi) (i64.const 0)))
        )
        ;; abs($a_hi)
        (select
            (i64.sub (i64.const 0) (i64.add (local.get $a_hi) (i64.extend_i32_u (i64.ne (local.get $a_lo) (i64.const 0)))))
            (local.get $a_hi)
            (local.get $negative)
        )
        ;; $b_lo
        (local.tee $negative (i32.wrap_i64 (local.get $b_lo)))

        ;; $negative is 1 if end result should be negative else 0
        (local.set $negative
            (i32.and (i64.lt_s (local.get $a_hi) (i64.const 0))
                     (i32.and (local.get $negative) (i32.const 1))
            )
        )

        (call $pow-inner)
        (local.set $a_hi)
        (local.set $a_lo)

        ;; overflow if result is negative
        (if (i64.lt_s (local.get $a_hi) (i64.const 0))
            (then (call $runtime-error (i32.const 0)))
        )

        (if (result i64 i64) (local.get $negative)
            (then
                (i64.sub (i64.const 0) (local.get $a_lo))
                (i64.sub (i64.const 0) (i64.add (local.get $a_hi) (i64.extend_i32_u (i64.ne (local.get $a_lo) (i64.const 0)))))
            )
            (else
                (local.get $a_lo)
                (local.get $a_hi)
            )
        )
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
    (export "sqrti-uint" (func $sqrti-uint))
    (export "sqrti-int" (func $sqrti-int))
    (export "bit-and-uint" (func $bit-and))
    (export "bit-and-int" (func $bit-and))
    (export "bit-not-uint" (func $bit-not))
    (export "bit-not-int" (func $bit-not))
    (export "bit-or-uint" (func $bit-or))
    (export "bit-or-int" (func $bit-or))
    (export "bit-xor-uint" (func $bit-xor))
    (export "bit-xor-int" (func $bit-xor))
    (export "bit-shift-left-uint" (func $bit-shift-left))
    (export "bit-shift-left-int" (func $bit-shift-left))
    (export "bit-shift-right-uint" (func $bit-shift-right-uint))
    (export "bit-shift-right-int" (func $bit-shift-right-int))
    (export "pow-uint" (func $pow-uint))
    (export "pow-int" (func $pow-int))
)
