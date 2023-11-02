;; This module contains a standard library for Clarity, defining Clarity's
;; builtins, to be called from the generated Wasm code.
(module
    (type (;0;) (func (param i32)))
    (type (;1;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64)))
    (type (;2;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i64 i64 i64 i64)))
    (type (;3;) (func (param i64 i64) (result i64 i64)))
    (type (;4;) (func (param i32 i32 i32) (result i32)))
    (type (;5;) (func (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)))
    (type (;6;) (func (param $offset i32) (param $length i32) (param $offset-result i32) (result i32 i32)))
    (type (;7;) (func (param $lo i64) (param $hi i64) (param $offset-result i32) (result i32 i32)))
    (type (;8;) (func (param $bool_in i32) (result i32)))
    (type (;9;) (func (param $offset_a i32) (param $length_a i32) (param $offset_b i32) (param $length_b i32) (result i32)))

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
    (import "clarity" "define_trait" (func $define_trait (param $name_offset i32)
                                                         (param $name_length i32)))
    (import "clarity" "impl_trait" (func $impl_trait (param $trait_offset i32)
                                                     (param $trait_length i32)))

    (import "clarity" "get_variable" (func $get_variable (param $name_offset i32)
                                                         (param $name_length i32)
                                                         (param $return_offset i32)
                                                         (param $return_length i32)))
    (import "clarity" "set_variable" (func $set_variable (param $name_offset i32)
                                                         (param $name_length i32)
                                                         (param $value_offset i32)
                                                         (param $value_length i32)))
    (import "clarity" "print" (func $print (param $value_offset i32)
                                           (param $value_length i32)))
    (import "clarity" "enter_as_contract" (func $enter_as_contract))
    (import "clarity" "exit_as_contract" (func $exit_as_contract))
    (import "clarity" "enter_at_block" (func $enter_at_block (param $block_hash_offset i32)
                                                             (param $block_hash_length i32)))
    (import "clarity" "exit_at_block" (func $exit_at_block))
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
    (import "clarity" "get_burn_block_info" (func $get_burn_block_info (param $name_offset i32)
                                                                       (param $name_length i32)
                                                                       (param $height_lo i64)
                                                                       (param $height_hi i64)
                                                                       (param $return_offset i32)
                                                                       (param $return_length i32)))
    (import "clarity" "contract_call" (func $contract_call (param $contract_id_offset i32)
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

    ;; (sha256) initial hash values: first 32 bits of the fractional parts of the square roots of the first 8 primes 2..19
    (data (i32.const 0) "\67\e6\09\6a\85\ae\67\bb\72\f3\6e\3c\3a\f5\4f\a5\7f\52\0e\51\8c\68\05\9b\ab\d9\83\1f\19\cd\e0\5b")

    ;; (sha256) K constants: first 32 bits of the fractional parts of the cube roots of the first 64 primes 2..311
    (data (i32.const 32) "\98\2f\8a\42\91\44\37\71\cf\fb\c0\b5\a5\db\b5\e9\5b\c2\56\39\f1\11\f1\59\a4\82\3f\92\d5\5e\1c\ab\98\aa\07\d8\01\5b\83\12\be\85\31\24\c3\7d\0c\55\74\5d\be\72\fe\b1\de\80\a7\06\dc\9b\74\f1\9b\c1\c1\69\9b\e4\86\47\be\ef\c6\9d\c1\0f\cc\a1\0c\24\6f\2c\e9\2d\aa\84\74\4a\dc\a9\b0\5c\da\88\f9\76\52\51\3e\98\6d\c6\31\a8\c8\27\03\b0\c7\7f\59\bf\f3\0b\e0\c6\47\91\a7\d5\51\63\ca\06\67\29\29\14\85\0a\b7\27\38\21\1b\2e\fc\6d\2c\4d\13\0d\38\53\54\73\0a\65\bb\0a\6a\76\2e\c9\c2\81\85\2c\72\92\a1\e8\bf\a2\4b\66\1a\a8\70\8b\4b\c2\a3\51\6c\c7\19\e8\92\d1\24\06\99\d6\85\35\0e\f4\70\a0\6a\10\16\c1\a4\19\08\6c\37\1e\4c\77\48\27\b5\bc\b0\34\b3\0c\1c\39\4a\aa\d8\4e\4f\ca\9c\5b\f3\6f\2e\68\ee\82\8f\74\6f\63\a5\78\14\78\c8\84\08\02\c7\8c\fa\ff\be\90\eb\6c\50\a4\f7\a3\f9\be\f2\78\71\c6")

    ;; (hash-160) selection of message word (r)
    (data (i32.const 288) "\00\01\02\03\04\05\06\07\08\09\0a\0b\0c\0d\0e\0f\07\04\0d\01\0a\06\0f\03\0c\00\09\05\02\0e\0b\08\03\0a\0e\04\09\0f\08\01\02\07\00\06\0d\0b\05\0c\01\09\0b\0a\00\08\0c\04\0d\03\07\0f\0e\05\06\02\04\00\05\09\07\0c\02\0a\0e\01\03\08\0b\06\0f\0d")

    ;; (hash-160) selection of message word (r')
    (data (i32.const 368) "\05\0e\07\00\09\02\0b\04\0d\06\0f\08\01\0a\03\0c\06\0b\03\07\00\0d\05\0a\0e\0f\08\0c\04\09\01\02\0f\05\01\03\07\0e\06\09\0b\08\0c\02\0a\00\04\0d\08\06\04\01\03\0b\0f\00\05\0c\02\0d\09\07\0a\0e\0c\0f\0a\04\01\05\08\07\06\02\0d\0e\00\03\09\0b")

    ;; (hash-160) rotate-left amount (s)
    (data (i32.const 448) "\0b\0e\0f\0c\05\08\07\09\0b\0d\0e\0f\06\07\09\08\07\06\08\0d\0b\09\07\0f\07\0c\0f\09\0b\07\0d\0c\0b\0d\06\07\0e\09\0d\0f\0e\08\0d\06\05\0c\07\05\0b\0c\0e\0f\0e\0f\09\08\09\0e\05\06\08\06\05\0c\09\0f\05\0b\06\08\0d\0c\05\0c\0d\0e\0b\08\05\06")

    ;; (hash-160) rotate-left amount (s')
    (data (i32.const 528) "\08\09\09\0b\0d\0f\0f\05\07\07\08\0b\0e\0e\0c\06\09\0d\0f\07\0c\08\09\0b\07\07\0c\07\06\0f\0d\0b\09\07\0f\0b\08\06\06\0e\0c\0d\05\0e\0d\0d\07\05\0f\05\08\0b\0e\0e\06\0e\06\09\0c\09\0c\05\0f\08\08\05\0c\09\0c\05\0e\06\08\0d\06\05\0f\0d\0b\0b")

    ;; (hash-160) K constants
    (data (i32.const 608) "\00\00\00\00\99\79\82\5a\a1\eb\d9\6e\dc\bc\1b\8f\4e\fd\53\a9")

    ;; (hash-160) K' constants
    (data (i32.const 628) "\e6\8b\a2\50\24\d1\4d\5c\f3\3e\70\6d\e9\76\6d\7a\00\00\00\00")

    ;; table that contains the 5 hash160 functions used during compression
    (type $hash160-compress-function (func (param i32 i32 i32 i32) (result i32)))
    (table 5 funcref) ;; table for hash160 compress function
    (elem (i32.const 0) $hash160-f1 $hash160-f2 $hash160-f3 $hash160-f4 $hash160-f5)

    ;; The error code is one of:
        ;; 0: overflow
        ;; 1: underflow
        ;; 2: divide by zero
        ;; 3: log of a number <= 0
        ;; 4: expected a non-negative number
        ;; 5: buffer to integer expects a buffer length <= 16
        ;; 6: panic
    (func $runtime-error (type 0) (param $error-code i32)
        ;; TODO: Implement runtime error
        unreachable
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

    (func $lt-buff (type 9) (param $offset_a i32) (param $length_a i32) (param $offset_b i32) (param $length_b i32) (result i32)
        (local $i i32) (local $sub i32)
        ;; pseudo-code:
        ;; let i = min(length_a, length_b)
        ;; while i != 0 {
        ;;   if ((sub = a[offset_a] - b[offset_b]) == 0) {
        ;;     offset_a += 1; offset_b += 1; i -= 1;
        ;;   } else { break }
        ;; }
        ;; return (sub != 0) ? (sub < 0) : (length_a < length_b)
        (block $done
            ;; we can skip the comparison loop if $i (min length) is 0
            (br_if $done
                (i32.eqz
                    (local.tee $i
                        ;; no i32.min in Wasm...
                        (select
                            (local.get $length_a)
                            (local.get $length_b)
                            (i32.lt_u (local.get $length_a) (local.get $length_b))
                        )
                    )
                )
            )
            (loop $loop
                (if
                    (i32.eqz
                        ;; $sub will be 0 if both are equal, otherwise its sign indicates which is smaller
                        (local.tee $sub
                            (i32.sub (i32.load8_u (local.get $offset_a)) (i32.load8_u (local.get $offset_b)))
                        )
                    )
                    (then
                        (local.set $offset_a (i32.add (local.get $offset_a) (i32.const 1)))
                        (local.set $offset_b (i32.add (local.get $offset_b) (i32.const 1)))
                        (br_if $loop (local.tee $i (i32.sub (local.get $i) (i32.const 1))))
                    )
                )
            )
        )
        ;; if sub is 0, it means that for the min length of both buffers, both are equals
        ;;   - in this case, the result is the comparison of the lengths
        ;;   - otherwise $sub < 0
        (select
            (i32.lt_s (local.get $sub) (i32.const 0))
            (i32.lt_u (local.get $length_a) (local.get $length_b))
            (local.get $sub)
        )
    )

    (func $gt-buff (type 9) (param $offset_a i32) (param $length_a i32) (param $offset_b i32) (param $length_b i32) (result i32)
        (local $i i32) (local $sub i32)
        ;; same algorithm as $lt-buff
        (block $done
            (br_if $done
                (i32.eqz
                    (local.tee $i
                        (select
                            (local.get $length_a)
                            (local.get $length_b)
                            (i32.lt_u (local.get $length_a) (local.get $length_b))
                        )
                    )
                )
            )
            (loop $loop
                (if
                    (i32.eqz
                        (local.tee $sub
                            (i32.sub (i32.load8_u (local.get $offset_a)) (i32.load8_u (local.get $offset_b)))
                        )
                    )
                    (then
                        (local.set $offset_a (i32.add (local.get $offset_a) (i32.const 1)))
                        (local.set $offset_b (i32.add (local.get $offset_b) (i32.const 1)))
                        (br_if $loop (local.tee $i (i32.sub (local.get $i) (i32.const 1))))
                    )
                )
            )
        )
        (select
            (i32.gt_s (local.get $sub) (i32.const 0))
            (i32.gt_u (local.get $length_a) (local.get $length_b))
            (local.get $sub)
        )
    )

    (func $le-buff (type 9) (param $offset_a i32) (param $length_a i32) (param $offset_b i32) (param $length_b i32) (result i32)
        (local $i i32) (local $sub i32)
        ;; same algorithm as $lt-buff
        (block $done
            (br_if $done
                (i32.eqz
                    (local.tee $i
                        (select
                            (local.get $length_a)
                            (local.get $length_b)
                            (i32.lt_u (local.get $length_a) (local.get $length_b))
                        )
                    )
                )
            )
            (loop $loop
                (if
                    (i32.eqz
                        (local.tee $sub
                            (i32.sub (i32.load8_u (local.get $offset_a)) (i32.load8_u (local.get $offset_b)))
                        )
                    )
                    (then
                        (local.set $offset_a (i32.add (local.get $offset_a) (i32.const 1)))
                        (local.set $offset_b (i32.add (local.get $offset_b) (i32.const 1)))
                        (br_if $loop (local.tee $i (i32.sub (local.get $i) (i32.const 1))))
                    )
                )
            )
        )
        (select
            (i32.le_s (local.get $sub) (i32.const 0))
            (i32.le_u (local.get $length_a) (local.get $length_b))
            (local.get $sub)
        )
    )

    (func $ge-buff (type 9) (param $offset_a i32) (param $length_a i32) (param $offset_b i32) (param $length_b i32) (result i32)
        (local $i i32) (local $sub i32)
        ;; same algorithm as $lt-buff
        (block $done
            (br_if $done
                (i32.eqz
                    (local.tee $i
                        (select
                            (local.get $length_a)
                            (local.get $length_b)
                            (i32.lt_u (local.get $length_a) (local.get $length_b))
                        )
                    )
                )
            )
            (loop $loop
                (if
                    (i32.eqz
                        (local.tee $sub
                            (i32.sub (i32.load8_u (local.get $offset_a)) (i32.load8_u (local.get $offset_b)))
                        )
                    )
                    (then
                        (local.set $offset_a (i32.add (local.get $offset_a) (i32.const 1)))
                        (local.set $offset_b (i32.add (local.get $offset_b) (i32.const 1)))
                        (br_if $loop (local.tee $i (i32.sub (local.get $i) (i32.const 1))))
                    )
                )
            )
        )
        (select
            (i32.ge_s (local.get $sub) (i32.const 0))
            (i32.ge_u (local.get $length_a) (local.get $length_b))
            (local.get $sub)
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

    (func $sha256-buf (type 6) (param $offset i32) (param $length i32) (param $offset-result i32) (result i32 i32)
        (local $i i32)
        ;; see this for an explanation: https://sha256algorithm.com/

        (call $extend-data (local.get $offset) (local.get $length))
        (local.set $length)

        (local.set $i (i32.const 0))
        (loop
            (call $block64 (local.get $i))
            (call $working-vars)
            (br_if 0
                (i32.lt_u
                    (local.tee $i (i32.add (local.get $i) (i32.const 64)))
                    (local.get $length)
                )
            )
        )

        ;; store at result position with correct endianness
        (v128.store
            (local.get $offset-result)
            (i8x16.swizzle
                (v128.load (global.get $stack-pointer))
                (v128.const i8x16 3 2 1 0 7 6 5 4 11 10 9 8 15 14 13 12)
            )
        )
        (v128.store offset=16
            (local.get $offset-result)
            (i8x16.swizzle
                (v128.load offset=16 (global.get $stack-pointer))
                (v128.const i8x16 3 2 1 0 7 6 5 4 11 10 9 8 15 14 13 12)
            )
        )

        (local.get $offset-result) (i32.const 32)
    )

    (func $sha256-int (type 7) (param $lo i64) (param $hi i64) (param $offset-result i32) (result i32 i32)
        ;; Copy data to the working stack, so that it has this relative configuration:
        ;;   0..32 -> Initial hash vals (will be the result hash in the end)
        ;;   32..288 -> Space to store W
        ;;   288..352 -> extended int
        (memory.copy (global.get $stack-pointer) (i32.const 0) (i32.const 32))

        (i64.store offset=288 (global.get $stack-pointer) (local.get $lo))
        (i64.store offset=296 (global.get $stack-pointer) (local.get $hi)) ;; offset = 288 + 8
        (i32.store offset=304 (global.get $stack-pointer) (i32.const 0x80)) ;; offset = 288+16
        (memory.fill (i32.add (global.get $stack-pointer) (i32.const 308)) (i32.const 0) (i32.const 46)) ;; offset = 288+20
        (i32.store8 offset=351 (global.get $stack-pointer) (i32.const 0x80)) ;; offset = 288+63

        (call $block64 (i32.const 0))
        (call $working-vars)

        (v128.store
            (local.get $offset-result)
            (i8x16.swizzle
                (v128.load (global.get $stack-pointer))
                (v128.const i8x16 3 2 1 0 7 6 5 4 11 10 9 8 15 14 13 12)
            )
        )
        (v128.store offset=16
            (local.get $offset-result)
            (i8x16.swizzle
                (v128.load offset=16 (global.get $stack-pointer))
                (v128.const i8x16 3 2 1 0 7 6 5 4 11 10 9 8 15 14 13 12)
            )
        )

        (local.get $offset-result) (i32.const 32)
    )

    (func $extend-data (param $offset i32) (param $length i32) (result i32)
        (local $res_len i32) (local $i i32) (local $len64 i64)
        ;; TODO: check if enough pages of memory and grow accordingly

        ;; Move data to the working stack, so that it has this relative configuration:
        ;;   0..32 -> Initial hash vals (will be the result hash in the end)
        ;;   32..288 -> Space to store W (result of $block64)
        ;;   288..$length+288 -> shifted data
        (memory.copy (global.get $stack-pointer) (i32.const 0) (i32.const 32))
        (memory.copy (i32.add (global.get $stack-pointer) (i32.const 288)) (local.get $offset) (local.get $length))

        (local.set $res_len ;; total size of data with expansion
            (i32.add
                (i32.or
                    ;; len + 8 bytes for the size
                    (i32.add (local.get $length) (i32.const 8))
                    (i32.const 0x3f)
                )
                (i32.const 1)
            )
        )

        ;; Add "1" at the end of the data
        (i32.store offset=288
            (i32.add (global.get $stack-pointer) (local.get $length))
            (i32.const 0x80)
        )
        ;; Fill the remaining part before the size with 0s
        (memory.fill
            (i32.add (i32.add (global.get $stack-pointer) (local.get $length)) (i32.const 289))
            (i32.const 0)
            (i32.sub (i32.sub (local.get $res_len) (local.get $length)) (i32.const 8))
        )

        ;; Add the size, as a 64bits big-endian integer
        (local.set $len64 (i64.extend_i32_u (i32.shl (local.get $length) (i32.const 3))))
        (i32.sub (i32.add (global.get $stack-pointer) (local.get $res_len)) (i32.const 8))
        (i64.or
            (i64.or
                (i64.or
                    (i64.shl (local.get $len64) (i64.const 0x38))
                    (i64.shl (i64.and (local.get $len64) (i64.const 0xff00)) (i64.const 0x28))
                )
                (i64.or
                    (i64.shl (i64.and (local.get $len64) (i64.const 0xff0000)) (i64.const 0x18))
                    (i64.shl (i64.and (local.get $len64) (i64.const 0xff000000)) (i64.const 0x8))
                )
            )
            (i64.or
                (i64.or
                    (i64.and (i64.shr_u (local.get $len64) (i64.const 0x8)) (i64.const 0xff000000))
                    (i64.and (i64.shr_u (local.get $len64) (i64.const 0x18)) (i64.const 0xff0000))
                )
                (i64.or
                    (i64.and (i64.shr_u (local.get $len64) (i64.const 0x28)) (i64.const 0xff00))
                    (i64.shr_u (local.get $len64) (i64.const 0x38))
                )
            )
        )
        i64.store offset=288

        (local.get $res_len)
    )

    (func $block64 (param $data i32)
        (local $origin i32)
        (local $i i32) (local $tmp i32)

        (local.set $origin (global.get $stack-pointer))
        (local.set $data (i32.add (local.get $origin) (local.get $data)))

        (local.set $i (i32.const 0))
        ;; copy first 64 bytes of data to offset as i32 with endianness adjustment
        ;; Using v128 to process more bytes at a time
        ;; TODO? : unroll this loop, since it's one instruction 4 times?
        (loop
            (i32.add (local.get $origin) (local.get $i))
            (i8x16.swizzle
                (v128.load offset=288 (i32.add (local.get $data) (local.get $i)))
                (v128.const i8x16 3 2 1 0 7 6 5 4 11 10 9 8 15 14 13 12)
            )
            v128.store offset=32

            (br_if 0
                (i32.lt_u
                    (local.tee $i (i32.add (local.get $i) (i32.const 16)))
                    (i32.const 64)
                )
            )
        )

        (local.set $i (i32.const 0))
        (loop
            (local.set $data (i32.add (local.get $origin) (local.get $i)))
            ;; store address: w(current+16)
            (i32.add (local.get $data) (i32.const 64))
            ;; w(current)
            (i32.load offset=32 (local.get $data))
            ;; sigma 0
            (local.set $tmp (i32.load offset=36 (local.get $data))) ;; offset = 32 + 4
            (i32.rotr (local.get $tmp) (i32.const 7))
            (i32.xor (i32.rotr (local.get $tmp) (i32.const 18)))
            (i32.xor (i32.shr_u (local.get $tmp) (i32.const 3)))
            i32.add
            ;; w(current+9)
            (i32.add (i32.load offset=68 (local.get $data))) ;; offset = 32+36
            ;; sigma 1
            (local.set $tmp (i32.load offset=88 (local.get $data))) ;; offset = 32+56
            (i32.rotr (local.get $tmp) (i32.const 17))
            (i32.xor (i32.rotr (local.get $tmp) (i32.const 19)))
            (i32.xor (i32.shr_u (local.get $tmp) (i32.const 10)))
            i32.add
            ;; save
            i32.store offset=32

            (br_if 0
                (i32.lt_u
                    (local.tee $i (i32.add (local.get $i) (i32.const 4)))
                    (i32.const 192)
                )
            )
        )
    )

    (func $working-vars
        (local $origin i32)
        (local $a i32) (local $b i32) (local $c i32) (local $d i32)
        (local $e i32) (local $f i32) (local $g i32) (local $h i32)
        (local $temp1 i32) (local $temp2 i32) (local $i i32)

        (local.set $origin (global.get $stack-pointer))

        (local.set $a (i32.load offset=0 (local.get $origin)))
        (local.set $b (i32.load offset=4 (local.get $origin)))
        (local.set $c (i32.load offset=8 (local.get $origin)))
        (local.set $d (i32.load offset=12 (local.get $origin)))
        (local.set $e (i32.load offset=16 (local.get $origin)))
        (local.set $f (i32.load offset=20 (local.get $origin)))
        (local.set $g (i32.load offset=24 (local.get $origin)))
        (local.set $h (i32.load offset=28 (local.get $origin)))

        (local.set $i (i32.const 0))
        (loop
        ;; compute $temp1: h + sigma1 + choice + k0 + w0
            (local.get $h) ;; h

            (i32.rotr (local.get $e) (i32.const 6))
            (i32.xor (i32.rotr (local.get $e) (i32.const 11)))
            (i32.xor (i32.rotr (local.get $e) (i32.const 25)))
            i32.add ;; + sigma1

            (i32.and (local.get $e) (local.get $f))
            (i32.xor (i32.and (i32.xor (local.get $e) (i32.const -1)) (local.get $g)))
            i32.add ;; + choice

            (i32.add (i32.load offset=32 (local.get $i))) ;; + k(current)

            (i32.add (i32.load offset=32 (i32.add (local.get $origin) (local.get $i)))) ;; + w(current)
            (local.set $temp1)

            ;; compute temp2: sigma0 + majority
            (i32.rotr (local.get $a) (i32.const 2))
            (i32.xor (i32.rotr (local.get $a) (i32.const 13)))
            (i32.xor (i32.rotr (local.get $a) (i32.const 22))) ;; sigma0

            (i32.and (local.get $a) (local.get $b))
            (i32.xor (i32.and (local.get $a) (local.get $c)))
            (i32.xor (i32.and (local.get $b) (local.get $c)))
            i32.add ;; + majority
            (local.set $temp2)

            ;; assign new variables
            (local.set $h (local.get $g))
            (local.set $g (local.get $f))
            (local.set $f (local.get $e))
            (local.set $e (i32.add (local.get $d) (local.get $temp1)))
            (local.set $d (local.get $c))
            (local.set $c (local.get $b))
            (local.set $b (local.get $a))
            (local.set $a (i32.add (local.get $temp1) (local.get $temp2)))

            (br_if 0
                (i32.lt_u
                    (local.tee $i (i32.add (local.get $i) (i32.const 4)))
                    (i32.const 256)
                )
            )
        )

        ;; update hash
        (i32.store offset=0 (local.get $origin) (i32.add (i32.load offset=0 (local.get $origin)) (local.get $a)))
        (i32.store offset=4 (local.get $origin) (i32.add (i32.load offset=4 (local.get $origin)) (local.get $b)))
        (i32.store offset=8 (local.get $origin) (i32.add (i32.load offset=8 (local.get $origin)) (local.get $c)))
        (i32.store offset=12 (local.get $origin) (i32.add (i32.load offset=12 (local.get $origin)) (local.get $d)))
        (i32.store offset=16 (local.get $origin) (i32.add (i32.load offset=16 (local.get $origin)) (local.get $e)))
        (i32.store offset=20 (local.get $origin) (i32.add (i32.load offset=20 (local.get $origin)) (local.get $f)))
        (i32.store offset=24 (local.get $origin) (i32.add (i32.load offset=24 (local.get $origin)) (local.get $g)))
        (i32.store offset=28 (local.get $origin) (i32.add (i32.load offset=28 (local.get $origin)) (local.get $h)))
    )

    (func $hash160-buf (type 6) (param $offset i32) (param $length i32) (param $offset-result i32) (result i32 i32)
        (local $i i32)
        ;; ripemd-160 article: https://www.esat.kuleuven.be/cosic/publications/article-317.pdf
        ;; Here we implement a ripemd with an easier padding since inputs are results of sha256,
        ;; and thus always have the same length.

        ;; move $stack-pointers: current value will contain sha256 result and moved place is new stack
        (global.set $stack-pointer (i32.add (local.tee $i (global.get $stack-pointer)) (i32.const 32)))
        ;; compute sha256
        (call $sha256-buf (local.get $offset) (local.get $length) (local.get $i))
        drop ;; we don't need the length of sha256, it is always 32
        (global.set $stack-pointer) ;; set $stack-pointer to its original value, aka offset of sha256 result

        (call $hash160-pad)
        (call $hash160-compress (local.get $offset-result))

        (local.get $offset-result) (i32.const 20)
    )

    (func $hash160-int (type 7) (param $lo i64) (param $hi i64) (param $offset-result i32) (result i32 i32)
        (local $i i32)
        ;; ripemd-160 article: https://www.esat.kuleuven.be/cosic/publications/article-317.pdf
        ;; Here we implement a ripemd with an easier padding since inputs are results of sha256,
        ;; and thus always have the same length.

        ;; move $stack-pointers: current value will contain sha256 result and moved place is new stack
        (global.set $stack-pointer (i32.add (local.tee $i (global.get $stack-pointer)) (i32.const 32)))
        ;; compute sha256
        (call $sha256-int (local.get $lo) (local.get $hi) (local.get $i))
        drop ;; we don't need the length of sha256, it is always 32
        (global.set $stack-pointer) ;; set $stack-pointer to its original value, aka offset of sha256 result

        (call $hash160-pad)
        (call $hash160-compress (local.get $offset-result))

        (local.get $offset-result) (i32.const 20)
    )

    (func $hash160-pad
        ;; MD-padding: (already placed sha256 +) "1" + "00000..." + size as i64 big endian
        (i64.store offset=32 (global.get $stack-pointer) (i64.const 0x80))
        (memory.fill 
            (i32.add (global.get $stack-pointer) (i32.const 40))
            (i32.const 0)
            (i32.const 16)
        )
        (i64.store offset=56 (global.get $stack-pointer) (i64.const 256))
    )

    (func $hash160-compress (param $offset-result i32)
        (local $h0 i32) (local $h1 i32) (local $h2 i32) (local $h3 i32) (local $h4 i32)
        (local $a1 i32) (local $b1 i32) (local $c1 i32) (local $d1 i32) (local $e1 i32)
        (local $a2 i32) (local $b2 i32) (local $c2 i32) (local $d2 i32) (local $e2 i32)
        (local $i i32) (local $round i32)

        (local.set $a2 (local.tee $a1 (local.tee $h0 (i32.const 0x67452301))))
        (local.set $b2 (local.tee $b1 (local.tee $h1 (i32.const 0xefcdab89))))
        (local.set $c2 (local.tee $c1 (local.tee $h2 (i32.const 0x98badcfe))))
        (local.set $d2 (local.tee $d1 (local.tee $h3 (i32.const 0x10325476))))
        (local.set $e2 (local.tee $e1 (local.tee $h4 (i32.const 0xc3d2e1f0))))

        (local.set $i (i32.const 0))
        (loop
            (local.set $round (i32.shr_u (local.get $i) (i32.const 4)))
            ;; ROUND
            ;; a
            (local.get $a1)
      
            ;; + f(round, b, c, d) + K(i)
            (local.get $b1) (local.get $c1) (local.get $d1)
            (i32.load offset=608 (i32.shl (local.get $round) (i32.const 2)))
            (call_indirect (type $hash160-compress-function) (local.get $round))
            i32.add

            ;; + word[r[i]]
            (i32.load
                (i32.add
                    (global.get $stack-pointer) 
                    (i32.shl (i32.load8_u offset=288 (local.get $i)) (i32.const 2))
                )
            )
            i32.add
      
            ;; left-rotate the addition by s[i]
            (i32.load8_u offset=448 (local.get $i))
            i32.rotl

            ;; + e
            (i32.add (local.get $e1))

            ;; set new vars
            (local.set $a1 (local.get $e1))
            (local.set $e1 (local.get $d1))
            (local.set $d1 (i32.rotl (local.get $c1) (i32.const 10)))
            (local.set $c1 (local.get $b1))
            (local.set $b1) ;; set with addition on the top of the stack

            ;; PARALLEL ROUND
            ;; a'
            (local.get $a2)

            ;; + f(round, b', c', d') + K'(i)
            (local.get $b2) (local.get $c2) (local.get $d2)
            (i32.load offset=628 (i32.shl (local.get $round) (i32.const 2)))
            (call_indirect (type $hash160-compress-function) (i32.sub (i32.const 4) (local.get $round)))
            i32.add

            ;; + word[r'[i]]
            (i32.load
                (i32.add
                    (global.get $stack-pointer) 
                    (i32.shl (i32.load8_u offset=368 (local.get $i)) (i32.const 2))
                )
            )
            i32.add

            ;; left-rotate the addition by s'[i]
            (i32.load8_u offset=528 (local.get $i))
            i32.rotl

            ;; + e'
            (i32.add (local.get $e2))

            ;; set new vars
            (local.set $a2 (local.get $e2))
            (local.set $e2 (local.get $d2))
            (local.set $d2 (i32.rotl (local.get $c2) (i32.const 10)))
            (local.set $c2 (local.get $b2))
            (local.set $b2) ;; set with addition on the top of the stack

            (br_if 0 
                (i32.lt_u 
                    (local.tee $i (i32.add (local.get $i) (i32.const 1)))
                    (i32.const 80)
                )
            )
        )

        ;; compute and save result to $offset-result
        (i32.store (local.get $offset-result) (i32.add (i32.add (local.get $h1) (local.get $c1)) (local.get $d2)))
        (i32.store offset=4 (local.get $offset-result) (i32.add (i32.add (local.get $h2) (local.get $d1)) (local.get $e2)))
        (i32.store offset=8 (local.get $offset-result) (i32.add (i32.add (local.get $h3) (local.get $e1)) (local.get $a2)))
        (i32.store offset=12 (local.get $offset-result) (i32.add (i32.add (local.get $h4) (local.get $a1)) (local.get $b2)))
        (i32.store offset=16 (local.get $offset-result) (i32.add (i32.add (local.get $h0) (local.get $b1)) (local.get $c2)))
    )

    (func $hash160-f1 (type $hash160-compress-function)
        (param $x i32) (param $y i32) (param $z i32) (param $k i32) (result i32)
        (i32.xor (i32.xor (local.get $x) (local.get $y)) (local.get $z))
        (i32.add (local.get $k))
    )

    (func $hash160-f2 (type $hash160-compress-function)
        (param $x i32) (param $y i32) (param $z i32) (param $k i32) (result i32)
        (i32.or
            (i32.and (local.get $x) (local.get $y))
            (i32.and (i32.xor (local.get $x) (i32.const -1)) (local.get $z))
        )
        (i32.add (local.get $k))
    )

    (func $hash160-f3 (type $hash160-compress-function)
        (param $x i32) (param $y i32) (param $z i32) (param $k i32) (result i32)
        (i32.xor
            (i32.or (local.get $x) (i32.xor (local.get $y) (i32.const -1)))
            (local.get $z)
        )
        (i32.add (local.get $k))
    )

    (func $hash160-f4 (type $hash160-compress-function)
        (param $x i32) (param $y i32) (param $z i32) (param $k i32) (result i32)
        (i32.or
            (i32.and (local.get $x) (local.get $z))
            (i32.and (local.get $y) (i32.xor (local.get $z) (i32.const -1)))
        )
        (i32.add (local.get $k))
    )

    (func $hash160-f5 (type $hash160-compress-function)
        (param $x i32) (param $y i32) (param $z i32) (param $k i32) (result i32)
        (i32.xor
            (local.get $x)
            (i32.or (local.get $y) (i32.xor (local.get $z) (i32.const -1)))
        )
        (i32.add (local.get $k))
    )

    (func $store-i32-be (param $address i32) (param $value i32)
        (i32.store 
            (local.get $address)
            (i32.or
                (i32.or
                    (i32.shl (local.get $value) (i32.const 24))
                    (i32.shl (i32.and (local.get $value) (i32.const 0xff00)) (i32.const 8))
                )
                (i32.or
                    (i32.and (i32.shr_u (local.get $value) (i32.const 8)) (i32.const 0xff00))
                    (i32.shr_u (local.get $value) (i32.const 24))
                )
            )
        )
    )
    
    (func $store-i64-be (param $address i32) (param $value i64)
        (i64.store 
            (local.get $address)
            (i64.or
                (i64.or
                    (i64.or
                        (i64.shl (local.get $value) (i64.const 56))
                        (i64.shl (i64.and (local.get $value) (i64.const 0xff00)) (i64.const 40))
                    )
                    (i64.or
                        (i64.shl (i64.and (local.get $value) (i64.const 0xff0000)) (i64.const 24))
                        (i64.shl (i64.and (local.get $value) (i64.const 0xff000000)) (i64.const 8))
                    )
                )
                (i64.or
                    (i64.or 
                        (i64.and (i64.shr_u (local.get $value) (i64.const 8)) (i64.const 0xff000000))
                        (i64.and (i64.shr_u (local.get $value) (i64.const 24)) (i64.const 0xff0000))
                    )
                    (i64.or
                        (i64.and (i64.shr_u (local.get $value) (i64.const 40)) (i64.const 0xff00))
                        (i64.shr_u (local.get $value) (i64.const 56))
                    )
                )
            )
        )
    )

    (func $buff-to-uint-be (param $offset i32) (param $length i32) (result i64 i64)
        (local $mask_lo i64) (local $mask_hi i64) (local $double v128)
        (if (i32.gt_u (local.get $length) (i32.const 16))
            (then (call $runtime-error (i32.const 5)))
        )

        (if (i32.eqz (local.get $length))
            (then (return (i64.const 0) (i64.const 0)))
        )

        ;; SAFETY: this function works because we already have data in the memory,
        ;;         otherwise we could have a negative offset.
        (local.set $offset (i32.sub (i32.add (local.get $offset) (local.get $length)) (i32.const 16)))

        ;; we compute masks for the low and high part of the resulting integer
        (local.set $mask_lo
            (select
                (i64.const -1)
                (local.tee $mask_hi
                    (i64.shr_u (i64.const -1) (i64.extend_i32_u (i32.and (i32.mul (local.get $length) (i32.const 56)) (i32.const 56))))
                )
                (i32.ge_u (local.get $length) (i32.const 8))
            )
        )
        (local.set $mask_hi (select (local.get $mask_hi) (i64.const 0) (i32.gt_u (local.get $length) (i32.const 8))))

        ;; we load both low and high part at once, and rearrange the bytes for endianness
        (local.set $double
            (i8x16.swizzle
                (v128.load (local.get $offset))
                (v128.const i8x16 7 6 5 4 3 2 1 0 15 14 13 12 11 10 9 8)
            )
        )

        (i64.and (i64x2.extract_lane 1 (local.get $double)) (local.get $mask_lo))
        (i64.and (i64x2.extract_lane 0 (local.get $double)) (local.get $mask_hi))
    )

    (func $buff-to-uint-le (param $offset i32) (param $length i32) (result i64 i64)
        (local $mask_lo i64) (local $mask_hi i64)
        (if (i32.gt_u (local.get $length) (i32.const 16))
            (then (call $runtime-error (i32.const 5)))
        )

        (if (i32.eqz (local.get $length))
            (then (return (i64.const 0) (i64.const 0)))
        )

        (local.set $mask_lo
            (select
                (i64.const -1)
                (local.tee $mask_hi
                    (i64.shr_u (i64.const -1) (i64.extend_i32_u (i32.and (i32.mul (local.get $length) (i32.const 56)) (i32.const 56))))
                )
                (i32.ge_u (local.get $length) (i32.const 8))
            )
        )
        (local.set $mask_hi (select (local.get $mask_hi) (i64.const 0) (i32.gt_u (local.get $length) (i32.const 8))))

        (i64.and (i64.load (local.get $offset)) (local.get $mask_lo))
        (i64.and (i64.load offset=8 (local.get $offset)) (local.get $mask_hi))
    )

    ;;
    ;; logical not implementation
    ;;
    (func $not (type 8) (param $bool_in i32) (result i32)
        (i32.eqz (local.get $bool_in))
    )

    ;;
    ;; 'is-eq-int' implementation
    ;;
    (func $stdlib.is-eq-int (type 5) (param $a_lo i64) (param $a_hi i64) (param $b_lo i64) (param $b_hi i64) (result i32)
        (i32.and
            (i64.eq (local.get $a_lo) (local.get $b_lo))
            (i64.eq (local.get $a_hi) (local.get $b_hi))
        )
    )

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
    (export "lt-buff" (func $lt-buff))
    (export "gt-buff" (func $gt-buff))
    (export "le-buff" (func $le-buff))
    (export "ge-buff" (func $ge-buff))
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
    (export "sha256-buf" (func $sha256-buf))
    (export "sha256-int" (func $sha256-int))
    (export "hash160-buf" (func $hash160-buf))
    (export "hash160-int" (func $hash160-int))
    (export "store-i32-be" (func $store-i32-be))
    (export "store-i64-be" (func $store-i64-be))
    (export "buff-to-uint-be" (func $buff-to-uint-be))
    (export "buff-to-uint-le" (func $buff-to-uint-le))
    (export "not" (func $not))
    (export "stdlib.is-eq-int" (func $stdlib.is-eq-int))
)
