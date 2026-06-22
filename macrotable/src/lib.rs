// Copyright 2026 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Compile-time repetition over short identifier lists.
//!
//! This crate provides two function-like procedural macros:
//!
//! - [`repeat!`] emits the whole body once per input row.
//! - [`splice!`] emits the body once and expands `#( ... )*` fragments inside it.
//!
//! Use these macros when the repetition is part of Rust syntax itself, such as
//! trait impls, match arms, struct fields, arrays, function arguments, or macro
//! arguments.
//!
//! # `repeat!`
//!
//! Use [`repeat!`] when every repeated block can stand on its own.
//!
//! ```rust
//! use macrotable::repeat;
//!
//! #[derive(Debug, PartialEq, Eq)]
//! enum MetricValue {
//!     Unsigned(u128),
//! }
//!
//! trait IntoMetricValue {
//!     fn into_metric_value(self) -> MetricValue;
//! }
//!
//! repeat!(#T in [u8, u16, u32, u64, usize] {
//!     impl IntoMetricValue for #T {
//!         fn into_metric_value(self) -> MetricValue {
//!             MetricValue::Unsigned(self as u128)
//!         }
//!     }
//! });
//!
//! assert_eq!(42u16.into_metric_value(), MetricValue::Unsigned(42));
//! assert_eq!(7usize.into_metric_value(), MetricValue::Unsigned(7));
//! ```
//!
//! Tuple bindings repeat over rows and can bind more than one placeholder:
//!
//! ```rust
//! use macrotable::repeat;
//!
//! #[derive(Debug, PartialEq, Eq)]
//! enum WireKind {
//!     Small,
//!     Large,
//! }
//!
//! trait WireType {
//!     fn wire_kind() -> WireKind;
//! }
//!
//! repeat!((#T, #Kind) in [(u16, Small), (u64, Large)] {
//!     impl WireType for #T {
//!         fn wire_kind() -> WireKind {
//!             WireKind::#Kind
//!         }
//!     }
//! });
//!
//! assert_eq!(<u16 as WireType>::wire_kind(), WireKind::Small);
//! assert_eq!(<u64 as WireType>::wire_kind(), WireKind::Large);
//! ```
//!
//! Use `_` in a tuple binding to skip a row value.
//!
//! # `splice!`
//!
//! Use [`splice!`] when the repeated tokens must fit inside one surrounding Rust
//! construct.
//!
//! ```rust
//! use macrotable::splice;
//!
//! struct WorkerStats {
//!     queued: usize,
//!     running: usize,
//!     failed: usize,
//! }
//!
//! impl WorkerStats {
//!     fn counters(&self) -> [(&'static str, usize); 3] {
//!         splice!(#field in [queued, running, failed] {
//!             [ #( (stringify!(#field), self.#field) ),* ]
//!         })
//!     }
//! }
//!
//! let stats = WorkerStats {
//!     queued: 4,
//!     running: 2,
//!     failed: 1,
//! };
//!
//! assert_eq!(
//!     stats.counters(),
//!     [("queued", 4), ("running", 2), ("failed", 1)]
//! );
//! ```
//!
//! `splice!` uses quote-style repetition syntax:
//!
//! ```rust,ignore
//! #( #name )*            // no separator
//! #( #name ),*           // comma separator
//! #( field: #name, )*    // punctuation in every repeated fragment
//! #( #key => #value, )*  // tuple rows work inside fragments
//! ```
//!
//! The token before `*` is used as the separator when it is written outside the
//! parenthesized fragment. Put punctuation inside the fragment when every
//! repeated item should carry it.
//!
//! Each `#( ... )*` fragment expands from the same input list:
//!
//! ```rust
//! use macrotable::splice;
//!
//! struct WorkerStats {
//!     queued: usize,
//!     running: usize,
//!     failed: usize,
//! }
//!
//! splice!(#field in [queued, running, failed] {
//!     const COUNTER_NAMES: &[&str] = &[
//!         #( stringify!(#field) ),*
//!     ];
//!
//!     fn empty_stats() -> WorkerStats {
//!         WorkerStats {
//!             #( #field: 0 ),*
//!         }
//!     }
//! });
//!
//! assert_eq!(COUNTER_NAMES, ["queued", "running", "failed"]);
//! assert_eq!(empty_stats().failed, 0);
//! ```
//!
//! In `splice!`, placeholders bound by the current invocation are only
//! available inside `#( ... )*`. The outer body is emitted once, so there is no
//! single current row outside those fragments.
//!
//! # Placeholders and Input
//!
//! Bind placeholders as `#name` and use them as `#name`. Bare identifiers are
//! left unchanged.
//!
//! Input values must be single identifier tokens. Paths, literals, generic
//! types, arrays, tuples, and grouped token fragments are not accepted as list
//! values. Use an alias when a repeated value stands for something more complex.
//!
//! ```rust,ignore
//! type UserId = crate::models::UserId;
//! type AccountId = crate::models::AccountId;
//!
//! repeat!(#Id in [UserId, AccountId] {
//!     impl MetricLabel for #Id {
//!         fn write_label(&self, out: &mut String) {
//!             out.push_str(&self.to_string());
//!         }
//!     }
//! });
//! ```
//!
//! # Nesting
//!
//! Nested invocations use normal macro expansion order. When an outer
//! [`repeat!`] expands, it substitutes matching placeholders anywhere in its
//! body, including nested macro input. Use distinct placeholder names at each
//! level.
//!
//! ```rust
//! use macrotable::{repeat, splice};
//!
//! struct WorkerStats {
//!     queued: usize,
//!     running: usize,
//! }
//!
//! trait FromStats {
//!     fn from_stats(stats: &WorkerStats) -> Self;
//! }
//!
//! repeat!(#T in [u64, usize] {
//!     impl FromStats for Vec<#T> {
//!         fn from_stats(stats: &WorkerStats) -> Self {
//!             splice!(#field in [queued, running] {
//!                 vec![ #( stats.#field as #T ),* ]
//!             })
//!         }
//!     }
//! });
//!
//! let stats = WorkerStats {
//!     queued: 4,
//!     running: 2,
//! };
//!
//! assert_eq!(<Vec<u64> as FromStats>::from_stats(&stats), vec![4, 2]);
//! assert_eq!(<Vec<usize> as FromStats>::from_stats(&stats), vec![4, 2]);
//! ```
//!
//! The outer `repeat!` replaces `#T` before the nested `splice!` invocation
//! runs.
//!
//! # Invocation Style
//!
//! The preferred style uses parentheses for the macro invocation and braces for
//! the repeated body:
//!
//! ```rust,ignore
//! repeat!(#T in [u8, u16] {
//!     impl IntoMetricValue for #T {
//!         // ...
//!     }
//! });
//! ```
//!
//! Add a trailing semicolon when the invocation appears in item or statement
//! position.
//!
//! # Errors
//!
//! The macros report compile errors for malformed input, including:
//!
//! - missing `#` in a binding,
//! - tuple pattern and tuple row arity mismatches,
//! - input values that are not single identifiers,
//! - current `splice!` placeholders used outside `#( ... )*`,
//! - `splice!` invocations without any `#( ... )*` repetition.

mod expand;
mod parse;

/// Repeats the whole body once for each input row.
///
/// See the crate-level documentation for syntax details, tuple bindings,
/// nesting behavior, and input restrictions.
#[proc_macro]
pub fn repeat(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::repeat(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Emits the body once and expands each `#( ... )*` fragment from the input rows.
///
/// See the crate-level documentation for repetition syntax, scope rules, and
/// nesting behavior.
#[proc_macro]
pub fn splice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::splice(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
