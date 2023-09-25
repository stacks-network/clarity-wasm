## Unreleased

### Feat

- implement `stx-transfer?`
- implement `stx-burn`
- add support for `stx-get-balance` and `stx-account`
- add support for `stx-`, `ft-`, and `nft-` expressions
- **clar2wasm**: increase the quantity of property tests
- add support for `as-contract`
- remove uses of `clarity-repl`
- move stacks-blockchain as submodule
- add support for builtin variables
- pass list into `fold-add-square` benchmark
- use runtime from blockchain repo for testing
- **WIP**: integrate with stacks-blockchain
- cleanup ownership
- Add bitwise functions to wasm_generator.rs
- implement bit-operations in standard library
- **clar2wasm**: add explanation comments in sqrti-uint function
- **clar2wasm**: add wasm implementation of sqrti for int
- **clar2wasm**: add wasm implementation of sqrti for uint
- **clar2wasm**: add traverse of sqrti function
- **clar2wasm**: name all unamed parameters in standard.wat functions
- **clar2wasm**: simplify log2 implementation
- **clar2wasm**: optimization of log2 algorithm + fix order in select statement
- **clar2wasm**: add log2-int implementation in standard
- **clar2wasm**: add log2-uint implementation in standard
- **clar2wasm**: traverse clarity log2 function
- **clar2wasm**: change generator of comparison operators to use visit instead of traverse
- **clar2wasm**: export all operators from standard.wat for int and uint
- **clar2wasm**: add standard.wat operators <, >, <= and >= for int
- **clar2wasm**: add standart.wat impl for uint for >, >= and <=
- add lt operator for uint
- **clar2wasm**: add the traverse function for comparison operators
- add support for `begin` expression
- convert Wasm values to Clarity Values

### Fix

- **clar2wasm**: handling of multiplication by i128::MIN
- **clar2wasm**: fix sqrti-int test
- **clar2wasm**: fix mul-int computation and overflow detection
- **clar2wasm**: fix wrong tests
- **clar2wasm**: learn how to do multipliactions and fix mul_uint in standard
- add placeholders for stdlib tests
- exclude stacks-blockchain from workspace
- mistake in previous lifetime cleanup
- fix clippy error
- set stack-pointer correctly
- **clar2wasm**: the standard did compile with wasmtime, but not wat2wasm
- **clar2wasm**: fix implementation of mul-uint
- fix all clippy warnings after rebase with main
- **tests**: fix test clippy warning
- **clar2wasm**: remove useless lifetime in ASTVisitor
- **clar2wasm**: reorder sqrt arguments after rebase on main
- **clar2wasm**: wasm add-uint fix for overflow condition + optimizations of locals
- **clar2wasm**: implementation of log2-uint used a xor instead of or
- **clar2wasm**: fix after rebase
- was considering the type of the cmp operator expression instead of its arguments
- Re-use the StacksConstant::default that was lost during rebase

### Refactor

- separate defines from normal execution
- **clar2wasm**: trim trailing whitespaces in standard.wat
- **clar2wasm**: replace a leading-zero call by a less than in standard.wat
- **clar2wasm**: try to make mul-int128 more readable
- **clar2wasm**: mul-int uses less variables
- remove tests boilerplate by macroizing the common parts

## v0.1 (2023-08-18)

### Feat

- clarinet compat
- Added WasmtimeHelper::new_from_str() and a little docs.
- add support for `(list ...)` and `(fold ...)`
- finish support for `var-set`
- read/write to global and contract context
- add support for data vars
- support creating a block in the Wasm
- add stack management
- add handler in visitor for modulo
- add division and modulus to stdlib
- implement 128-bit multiply with overflow detection
- build standard.wasm as part of build process
- support Clarity 128-bit arithmetic
- support > 2 operands in arithmetic ops
- support calling user-defined functions
- add support for `response` and public funcs
- add cargo config for easy install
- add support for atoms and read-only funcs
- add `define-private` and cleanup top-level

### Fix

- resolve issues in tests
- fold had the operands in the wrong order
- last rebase broke benchmark imports
- clippy warning about "too_many_args" was wrong in this case
- clippy + fmt
- remove lifetime for ClarityWasmResult and use Vec instead of reference
- fix error in function creation
- handle missing overflow cases for `mul-int128`
- define `runtime-error` in stdlib
- remove `local_funcs` and use `module.funcs`
- add function name to private func

### Refactor

- **tests**: put tests in a test folder and all lib function in lib.rs
- pass settings into `compile`
- define `FunctionContext`
- change the way the results are returned
