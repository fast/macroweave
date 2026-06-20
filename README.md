# macro-template

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/macro-template.svg
[crates-url]: https://crates.io/crates/macro-template
[docs-badge]: https://img.shields.io/docsrs/macro-template
[docs-url]: https://docs.rs/macro-template
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[license-badge]: https://img.shields.io/crates/l/macro-template
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/macro-template/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/macro-template/actions?query=workflow%3ACI

<!-- macro-template-docs-start -->

macro-template provides `template!`, a procedural macro for generating repeated Rust code from compact, table-driven inputs.

## Motivation

`macro-template` resulted from a ScopeDB code refactor. ScopeDB has used [`match-template`](https://github.com/tisonkun/match-template/) for variant/type match arms and [`macro_find_and_replace`](https://github.com/lord-ne/rust-macro-find-and-replace/) for repeating Rust fragments over type lists. While replacing them, I found that I wanted the same thing in both places: write the choices once, name the columns, and use those names in Rust syntax. There was no existing macro fitting that shape.

That is the table-driven case `template!` is built around:

```rust
trait ReadLe {
    fn read_le(input: &[u8]) -> Self;
}

macro_template::template! {
    for (Ty, Width) in [
        (u16, 2),
        (u32, 4),
        (u64, 8),
    ] {
        impl ReadLe for Ty {
            fn read_le(input: &[u8]) -> Self {
                Ty::from_le_bytes(input[..Width].try_into().unwrap())
            }
        }
    }
}

assert_eq!(u16::read_le(&[0x34, 0x12]), 0x1234);
assert_eq!(u32::read_le(&[1, 0, 0, 0]), 1);
```

When looking for existing approaches, I also found [`seq-macro`](https://github.com/dtolnay/seq-macro), which covers a neighboring repetition pattern: range-driven generation, where `N in 0..=2` becomes literal tokens like `0`, `1`, and `2`. `template!` keeps both forms under one syntax: table rows, ranges, `#( ... )*` for partial repetition, and multiple `for` clauses for Cartesian products. The examples below expand each case.

## Examples

### Whole-body repetition

Without splice syntax, the whole template body is repeated once per input row:

```rust
trait TypeName {
    const NAME: &'static str;
}

macro_template::template! {
    for (Ty, Name) in [
        (u8, "u8"),
        (u16, "u16"),
        (u32, "u32"),
    ] {
        impl TypeName for Ty {
            const NAME: &'static str = Name;
        }
    }
}

assert_eq!(<u16 as TypeName>::NAME, "u16");
```

### Partial repetition

When only part of a surrounding construct should repeat, put that part in `#( ... )*`. A single separator token tree can be written before `*`, such as `#( ... ),*` for comma-separated output:

```rust
fn keyword_code(text: &str) -> Option<u8> {
    macro_template::template! {
        for (Pat, Code) in [
            ("async", 1u8),
            ("await", 2u8),
        ] {
            match text {
                #(Pat => Some(Code)),*,
                _ => None,
            }
        }
    }
}

assert_eq!(keyword_code("async"), Some(1));
assert_eq!(keyword_code("await"), Some(2));
assert_eq!(keyword_code("fn"), None);
```

When a template contains `#( ... )*` or `#( ... ),*`, template variables are substituted only inside the splice body, and the surrounding tokens are emitted once. Surrounding identifiers stay literal, even when they have the same name as a template variable. If a value should vary, place it in the splice body.

`#( ..., )*` and `#( ... ),*` are different: the latter does not produce a trailing comma. This matches delimiter repetition in `macro_rules!`.

### Range inputs

Inputs can also be ranges of integers, characters, or bytes. Range inputs are written directly after `in`, without surrounding brackets. Wrap the range in parentheses when calling a range method such as `.rev()`:

```rust
let tuple = ("red", "green", "blue");
let mut fields = vec![];

macro_template::template! {
    for N in (0..3).rev() {
        fields.push(tuple.N);
    }
}

assert_eq!(fields, vec!["blue", "green", "red"]);
```

This cannot be written using an ordinary for-loop because elements of a tuple can only be accessed by their integer literal index, not by a variable.

Integer ranges preserve the radix, suffix, and shared padding width from their bounds. `.strip_prefix()` removes the radix prefix before substitution, which is useful when combining range values with [`paste`](https://docs.rs/paste/) for identifier generation:

```rust
macro_template::template! {
    for N in (0x00A..=0x00C).strip_prefix() {
        paste::paste! {
            enum Pin {
                #( [<Pin N>], )*
            }
        }
    }
}

let _ = (Pin::Pin00A, Pin::Pin00B, Pin::Pin00C);
```

### Cartesian products

Multiple input clauses form a Cartesian product in clause order. This is useful when two or more independent dimensions share the same generated body:

```rust
struct Cpu;
struct Gpu;

trait Kernel<T> {
    fn run(input: T) -> T;
}

macro_template::template! {
    for Backend in [Cpu, Gpu],
    for Ty in [f32, f64],
    {
        impl Kernel<Ty> for Backend {
            fn run(input: Ty) -> Ty {
                input
            }
        }
    }
}

assert_eq!(<Gpu as Kernel<f64>>::run(1.5), 1.5);
```

<!-- macro-template-docs-end -->

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The current policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `crate 1.0` requires Rust 1.85.0, then `crate 1.0.z` for all values of `z` will also require Rust 1.85.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
