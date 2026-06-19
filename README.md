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

## Overview

macro-template is a procedural macro that generates repeated Rust code in multiple places with table-driven sources.

For example, the following code generates `From` implementations for `MyEnum` from all its variants:

```rust
enum MyEnum {
    A(MyStructA),
    B(MyStructB),
    C(MyStructC),
}

macro_template::template! {
    for (Variant, Type) in [(A, MyStructA), (B, MyStructB), (C, MyStructC)] {
        impl From<Type> for MyEnum {
            fn from(value: Type) -> Self {
                MyEnum::Variant(value)
            }
        }
    }
}
```

You can iterate over a matrix as well:

```rust
macro_template::template! {
    for (Endian, Method) in [
        (LittleEndian, to_le_bytes),
        (BigEndian, to_be_bytes),
        (NativeEndian, to_ne_bytes)
    ],
    for (Ty, Width) in [
        (u16, 2),
        (u32, 4),
    ],
    {
        impl StoreBytes<Endian, Width> for Ty {
            fn store_bytes(&self) -> [u8; Width] {
                self.Method()
            }
        }
    }
}
```

Or, you can generate repeated match arms by pattern:

```rust
macro_template::template! {
    for T in [Int, Real, Double] {
        match Foo {
            @splice { EvalType::T => { panic!("{}", EvalType::T); }, }
            EvalType::Other => unreachable!(),
        }
    }
}
```

Naturally, if the match arm differs left-hand side and right-hand side:

```rust
macro_template::template! {
    for (K, Value) in [
        (Apple, "apple"),
        (Banana, "banana"),
        (Cherry, "cherry"),
    ] {
        match kind {
            @splice { Kind::K => { println!("{}", Value); }, }
        }
    }
}
```

The iterator can be a sequence of numbers, characters, or bytes:

```rust
// sequential numeric counter
let tuple = (1000, 100, 10);
let mut sum = 0;
macro_template::template! {
    for i in [0..3] {
        sum += tuple.i;
    }
}
assert_eq!(sum, 1110);

// sequential character collector
let mut string = String::new();
macro_template::template! {
    for c in ['x'..='z'] {
        string.push(c);
    }
}
assert_eq!(string, "xyz");
```

You can combine multiple iterators in a single template:

```rust
let mut values = Vec::new();

macro_template::template! {
    for Prefix in ["read", "write"],
    for Code in 200..=201 {
        values.push((Prefix, Code));
    }
}

assert_eq!(
    values,
    [("read", 200), ("read", 201), ("write", 200), ("write", 201)],
);
```

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The current policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `crate 1.0` requires Rust 1.85.0, then `crate 1.0.z` for all values of `z` will also require Rust 1.85.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).

## Origins
