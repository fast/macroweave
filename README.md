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
compile-time repetitions over identifier lists.

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

## Examples

### Whole-body repetition

Use `repeat!` when every repeated block can stand on its own:

```rust
use macrotable::repeat;

#[derive(Debug, PartialEq, Eq)]
enum MetricValue {
    Unsigned(u128),
}

trait IntoMetricValue {
    fn into_metric_value(self) -> MetricValue;
}

repeat!(#T in [u8, u16, u32, u64, usize] {
    impl IntoMetricValue for #T {
        fn into_metric_value(self) -> MetricValue {
            MetricValue::Unsigned(self as u128)
        }
    }
});

assert_eq!(42u16.into_metric_value(), MetricValue::Unsigned(42));
```

### Partial repetition

Use `splice!` when repeated pieces must fit inside one surrounding Rust
construct:

```rust
use macrotable::splice;

struct WorkerStats {
    queued: usize,
    running: usize,
    failed: usize,
}

impl WorkerStats {
    fn counters(&self) -> [(&'static str, usize); 3] {
        splice!(#field in [queued, running, failed] {
            [ #( (stringify!(#field), self.#field) ),* ]
        })
    }
}

let stats = WorkerStats {
    queued: 4,
    running: 2,
    failed: 1,
};

assert_eq!(
    stats.counters(),
    [("queued", 4), ("running", 2), ("failed", 1)]
);
```

`#( ... ),*` repeats without a trailing comma. Put the comma inside the
fragment, as in `#( ..., )*`, when every repeated item should carry it.

Run the complete example with:

```sh
cargo run --example metrics
```

## Rules

- Bind placeholders as `#name` and use them as `#name`.
- Bare identifiers are left unchanged.
- Input values must be single identifiers. Alias complex types first.
- Tuple rows can bind multiple placeholders, and `_` skips a row value.
- In `splice!`, placeholders from the current invocation are only available
  inside `#( ... )*`.
- Nested invocations are supported. Use different placeholder names at each
  level.

See the crate documentation for full syntax and error behavior.

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The minimum Rust version can be increased in minor version updates. For example,
if crate `1.0` requires Rust 1.85.0, then all `1.0.z` releases also support Rust
1.85.0 or newer, while `1.y` for `y > 0` may require a newer compiler.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
