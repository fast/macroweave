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

macro-template is a procedural macro that generates repeated Rust code in multiple places with table-driven inputs.

## Motivations

This crate is inspired by [`match-template`](https://github.com/tisonkun/match-template/) and [`macro_find_and_replace`](https://github.com/lord-ne/rust-macro-find-and-replace).

When developing ScopeDB, we introduced these two proc-macros to generate repeated code for match arms and impls. I noticed that they share a common pattern: iterating over a table of values and generating code based on it. I wanted to unify these patterns into a single, concise, but flexible macro that can handle various use cases. That's how `macro-template` was born.

Last but not least, I found [`seq-macro`](https://github.com/dtolnay/seq-macro) and borrowed some ideas from it, such as iterating over a range of numbers, characters, or bytes, and using a syntax `@splice { }` (equivalent to `seq-macro`'s or `quote`'s `#( ... )*`) to generate partial repeated substitutions. This eliminates the need for an extra `template_match!` to handle repetitions in match arms, and allows for more flexible code generation.

Go back to the beginning, why do you need `macro_template:template!` at all? Isn't it the same as a simple `macro_rules!`?

```rust
macro_rules! impl_serialize {
    ($($ty:ty),* $(,)?) => {
        $(
            impl serde_core::Serialize for BSize<$ty> {
                fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
                where
                    S: serde_core::Serializer,
                {
                    if ser.is_human_readable() {
                        ser.collect_str(self)
                    } else {
                        self.0.serialize(ser)
                    }
                }
            }
        )*
    };
}

impl_serialize!(u8, u16, u32, u64, usize);
```

Except that `macro_template:template!` supports more flexible substitution patterns as shown in the [Examples](#examples) section, `template!` has a concise syntax, and it saves you from declaring an extra `macro_rules!` (Naming It!) and invoking it.

The example above can be rewritten as:

```rust
macro_template::template! {
    for Ty in [u8, u16, u32, u64, usize] {
        impl serde_core::Serialize for BSize<Ty> {
            fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
            where
                S: serde_core::Serializer,
            {
                if ser.is_human_readable() {
                    ser.collect_str(self)
                } else {
                    self.0.serialize(ser)
                }
            }
        }
    }
}
```

## Examples

Firstly, you can generate code with a template and a matrix of values:

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

Or, you can do substitutions only partially with `@splice`. For example, to generate match arms:

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

When a template contains `@splice`, template variables are substituted only inside `@splice { ... }`. Surrounding tokens stay literal, even when an identifier has the same name as a template variable. If a value should vary, place it in the splice block.

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
let mut values = vec![];

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
