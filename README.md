# macrotable

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/macrotable.svg
[crates-url]: https://crates.io/crates/macrotable
[docs-badge]: https://img.shields.io/docsrs/macrotable
[docs-url]: https://docs.rs/macrotable
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[license-badge]: https://img.shields.io/crates/l/macrotable
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/macro-template/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/macro-template/actions?query=workflow%3ACI

`macrotable` provides two function-like procedural macros for small
compile-time repetitions over comma-separated Rust token sequences.

- `repeat!` emits the whole body once per input row.
- `splice!` emits the body once and expands `#( ... )*` fragments inside it.

## Install

```toml
[dependencies]
macrotable = "0.1"
```

## Motivation

Use `macrotable` when the repetition is Rust syntax, not runtime control flow.
Typical cases include repeated trait impls, match arms, enum variants, arrays,
function arguments, and macro arguments.

Function-like macros are a good fit when the repeated code does not belong to
one item that an attribute or derive macro can annotate.

Each list entry can contain one or more Rust tokens, such as a path, type,
expression, or grouped syntax. Separate entries with commas at the list level.

## Examples

### Whole-body repetition

Use `repeat!` when each expansion is a complete item or statement:

```rust
use macrotable::repeat;

#[derive(Debug, PartialEq, Eq)]
enum QueryValue {
    Signed(i128),
    Unsigned(u128),
}

repeat!((T, Variant, Out) in [
    (i8, Signed, i128),
    (i16, Signed, i128),
    (u8, Unsigned, u128),
    (u16, Unsigned, u128),
] {
    impl From<T> for QueryValue {
        fn from(value: T) -> Self {
            QueryValue::Variant(value as Out)
        }
    }
});

assert_eq!(QueryValue::from(-7i16), QueryValue::Signed(-7));
assert_eq!(QueryValue::from(42u16), QueryValue::Unsigned(42));
```

### Partial repetition

Use `splice!` when one Rust construct needs repeated pieces inside it:

```rust
use macrotable::splice;

splice!(Variant in [Build, Test, Publish] {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Command {
        #( Variant, )*
    }

    impl Command {
        fn as_str(self) -> &'static str {
            match self {
                #( Command::Variant => stringify!(Variant), )*
            }
        }
    }
});

assert_eq!(Command::Build.as_str(), "Build");
assert_eq!(Command::Publish.as_str(), "Publish");
```

`#( ... ),*` repeats without a trailing comma. Put the comma inside the
fragment, as in `#( ..., )*`, when every repeated item should carry it.

Run a complete example that combines both macros:

```sh
cargo run --example metrics
```

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The minimum Rust version can be increased in minor version updates. For example,
if crate `1.0` requires Rust 1.85.0, then all `1.0.z` releases also support Rust
1.85.0 or newer, while `1.y` for `y > 0` may require a newer compiler.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
