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

`macro-template` came out of a ScopeDB code refactor where two procedural macros were doing almost the same job from different repetition patterns: [`match-template`](https://github.com/tisonkun/match-template/) generated match arms from variant/type mappings, while [`macro_find_and_replace`](https://github.com/lord-ne/rust-macro-find-and-replace/) repeated a Rust fragment after replacing one token with each type in a list.

The common shape was simpler than either macro's DSL: write down a small table, bind one or more identifiers to each row, and expand ordinary Rust tokens with those bindings. Later, [`seq-macro`](https://github.com/dtolnay/seq-macro) made the same pattern visible for ranges: bind an identifier to each number, byte, or character, then expand a fragment, with `#( ... )*` for the part that repeats inside a surrounding item.

`template!` is that model directly: `for (Variant, Ty) in [...] { ... }`, optional `#( ... )*` when only part of the body repeats, and multiple `for` clauses when a matrix of combinations is what you need. The point is not another clever mini-language; it is a table-driven form that still reads like the Rust it generates.

## Examples

The examples below cover whole-body repetition, partial repetition, ranges, and multidimensional inputs.

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

Inputs can also be ranges of integers, characters, or bytes. Range inputs are written directly after `in`, without surrounding brackets:

```rust
let tuple = (1000, 100, 10);
let mut sum = 0;

macro_template::template! {
    for N in 0..3 {
        sum += tuple.N;
    }
}

assert_eq!(sum, 1110);

let mut chars = String::new();

macro_template::template! {
    for C in 'x'..='z' {
        chars.push(C);
    }
}

assert_eq!(chars, "xyz");
```

Integer ranges preserve the radix, suffix, and shared padding width from their bounds.

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
