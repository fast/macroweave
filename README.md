# macroweave

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/macroweave.svg
[crates-url]: https://crates.io/crates/macroweave
[docs-badge]: https://img.shields.io/docsrs/macroweave
[docs-url]: https://docs.rs/macroweave
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[license-badge]: https://img.shields.io/crates/l/macroweave
[license-url]: https://www.apache.org/licenses/LICENSE-2.0
[actions-badge]: https://github.com/fast/macroweave/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/macroweave/actions?query=workflow%3ACI

<!-- macroweave-docs-start -->

`macroweave` provides procedural macros for generating repeated Rust code from compact, table-driven inputs.

## Motivation

`macroweave` is for repetition that has to become Rust syntax, not runtime control flow. You write the choices once, name the columns, and use those names in Rust syntax.

That is the table-driven case `macroweave` is built around:

```rust
use macroweave::repeat;

trait ReadLe {
    fn read_le(input: &[u8]) -> Self;
}

repeat!((Ty, Width) in [
    (u16, 2),
    (u32, 4),
    (u64, 8),
] {
    impl ReadLe for Ty {
        fn read_le(input: &[u8]) -> Self {
            Ty::from_le_bytes(input[..Width].try_into().unwrap())
        }
    }
});

assert_eq!(u16::read_le(&[0x34, 0x12]), 0x1234);
assert_eq!(u32::read_le(&[1, 0, 0, 0]), 1);
```

This cannot be written as an ordinary for-loop because `Ty` and `Width` need to be substituted as tokens before the generated code is type-checked.

# Whole-body repetition

Without splice syntax, [`repeat!`] emits the whole body once per input row:

```rust
use macroweave::repeat;

trait TypeName {
    const NAME: &'static str;
}

repeat!((Ty, Name) in [
    (u8, "u8"),
    (u16, "u16"),
    (u32, "u32"),
] {
    impl TypeName for Ty {
        const NAME: &'static str = Name;
    }
});

assert_eq!(<u8 as TypeName>::NAME, "u8");
assert_eq!(<u16 as TypeName>::NAME, "u16");
assert_eq!(<u32 as TypeName>::NAME, "u32");
```

# Partial repetition

When only part of a surrounding construct should repeat, use [`splice!`] and put that part in `#( ... )*`. A single separator can be written before `*`, such as `#( ... ),*` for comma-separated output:

```rust
use macroweave::splice;

fn keyword_code(text: &str) -> Option<u8> {
    splice!((Pat, Code) in [
        ("async", 1u8),
        ("await", 2u8),
    ] {
        match text {
            #(Pat => Some(Code)),*,
            _ => None,
        }
    })
}

assert_eq!(keyword_code("async"), Some(1));
assert_eq!(keyword_code("await"), Some(2));
assert_eq!(keyword_code("fn"), None);
```

Placeholders are substituted only inside the splice body, and the surrounding tokens are emitted once. Surrounding identifiers stay literal, even when they have the same name as a placeholder. If a value should vary, place it in the splice body.

`#( ..., )*` and `#( ... ),*` are different: the latter does not produce a trailing comma. This matches delimiter repetition in `macro_rules!`.

# Syntax notes

- Bind placeholders as bare identifiers, such as `Ty` or `Name`.
- Tuple rows bind multiple placeholders, and `_` skips a row value.
- Row values can contain one or more Rust tokens. Top-level commas separate rows.
- Nested invocations are supported. Use different placeholder names at each level.

<!-- macroweave-docs-end -->

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The minimum Rust version can be increased in minor version updates. For example, if crate `1.0` requires Rust 1.85.0, then all `1.0.z` releases also support Rust 1.85.0 or newer, while `1.y` for `y > 0` may require a newer compiler.

## License

This project is licensed under [Apache License, Version 2.0][license-url].

## Origins

`macroweave` resulted from a ScopeDB code refactor. ScopeDB has used [`match-template`](https://github.com/tisonkun/match-template/) for variant/type match arms and [`macro_find_and_replace`](https://github.com/lord-ne/rust-macro-find-and-replace/) for repeating Rust fragments over type lists. While reviewing their macro usages, I found that I wanted the same thing in both places: write the choices once, name the columns, and use those names in Rust syntax. There was no existing solution fitting that shape.
