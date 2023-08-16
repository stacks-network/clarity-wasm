       /     /   ▶ clarity-wasm
      / --- /      Compile Clarity to Wasm.
     /     /       Generate WebAssembly from your Clarity code for fast and portable execution.

<div align="center">

[![Introduction](https://img.shields.io/badge/%23-%20Introduction%20-orange?labelColor=gray)](#introduction) [![Features](https://img.shields.io/badge/%23-Features-orange?labelColor=gray)](#features) [![Quick Start](https://img.shields.io/badge/%23-Quick%20Start-orange?labelColor=gray)](#quick-start) [![Documentation](https://img.shields.io/badge/%23-Documentation-orange?labelColor=gray)](#documentation) [![Contribute](https://img.shields.io/badge/%23-Contribute-orange?labelColor=gray)](#contribute)

</div>

---

## Introduction

`clar2wasm` is a compiler for generating [WebAssembly](https://webassembly.org/) from [Clarity](https://github.com/clarity-lang/reference).

## Features

## Quick-start

### Command line tool

Install the command line tool, `clar2wasm` with:

```sh
cargo clar2wasm-install
```

Once installed, try compiling one of our examples:

```sh
clar2wasm examples/define-read-only-0.clar
```

This will generate a wasm file, `examples/define-read-only-0.wasm`, from the Clarity source code.

You can view the text format of the generated Wasm by using a tool like [`wasm2wat`](https://github.com/WebAssembly/wabt):

```sh
wasm2wat examples/define-read-only-0.wasm
```

The output should look something like this:

```wasm
(module
  (type (;0;) (func))
  (type (;1;) (func (result i64)))
  (func $simple (type 1) (result i64)
    i64.const 42)
  (func (;1;) (type 0)
    return)
  (export "simple" (func $simple))
  (export ".top-level" (func 1)))
```

### Crate

`clar2wasm` is also available as a Rust library crate, to embed into other Rust projects.

## Documentation

### Top-Level Expressions

Any top-level expressions from a Clarity contract are added into a `.top-level` function that is exported from the generated Wasm module. This function should be called once during contract deployment.

### ABI

WebAssembly only supports basic number types, `i32`, `i64`, `f32`, and `f64`. We need to decide how to map Clarity types into these Wasm types.

- `int`: pair of `i64`s (upper, lower)
- `uint`: pair of `i64`s (upper, lower)
- `bool`: `i32`
- `principal`: `i32` pointer to stack; stack contains 20 bytes for standard principal followed by an `i32` indicating the length of the contract name, which, if non-zero, is followed by the contract name string.
- `buff`: `i32` pointer to stack, `i32` length
- `string-ascii`: `i32` pointer to stack, `i32` length
- `string-utf8`: `i32` pointer to stack, `i32` length
- `list`: `i32` pointer to stack, `i32` length
- `tuple`: each value in the tuple concatenated
- `optional`: `i32` indicator (`0` for `none`, `1` for `some`), followed by value for `some`
- `response`: `i32` indicator (`0` for `err`, `1` for `ok`) followed by ok value, then err value

### Memory Management

The types indicated above that are represented by a pointer to the stack need to actually be allocated space on the stack, and then the stack needs to be properly managed for each function. A global `$stack-pointer` is defined in the standard library. This value points to the top of the stack. Function locals are allocated on the top of the stack, and in the function postlude, the stack pointer is set back to its initial position.

### Standard Library

Certain Clarity operations are implemented as functions in [_standard.wat_](src/standard/standard.wat). This text format is then used during the build process to generate _standard.wasm_ which gets loaded into `clar2wasm`. Any operations that are cleaner to implement as a function call instead of directly generating Wasm instructions go into this library. For example, you can find the Clarity-style implementation of arithmetic operations in this library. These need to be written out manually because WebAssembly only supports 64-bit integers. The library implements 128-bit arithmetic, with the overflow checks that Clarity requires.

### Host Interface

When executing the compiled Clarity code, it needs to interact with the host - for example reading/writing to the MARF, emitting events, etc. We define a host interface that the generated Wasm code can call to perform these operations. Since these functions are type-agnostic, values are passed back and forth on the stack. The host function is responsible for marshalling/unmarshalling values to/from the Wasm format as needed (see ABI section above). These functions are imported by the standard library module, and it is the responsibility of the host to provide implementations of them.

| Clarity Operation | Host Function | Inputs | Outputs |
| --- | --- | --- | --- |
| `var-get` | `get_variable` | - ` var_name`: string (offset: i32, length: i32) | - |
|  |  | - `result`: stack pointer (offset: i32, length: i32) |  |
| `var-set` | `set_variable` | - `var_name`: string (offset: i32, length: i32) | - |
|  |  | - `value`: stack pointer (offset: i32, length: i32) |  |

## Contribute
