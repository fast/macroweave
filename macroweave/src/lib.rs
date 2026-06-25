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

//! Procedural macros for generating repeated Rust code from compact,
//! table-driven inputs.
//!
//! `macroweave` is for repetition that has to become Rust syntax, not runtime
//! control flow. You write the choices once, name the columns, and use those
//! names in Rust syntax.
//!
//! That is the table-driven case `macroweave` is built around:
//!
//! ```rust
//! use macroweave::repeat;
//!
//! trait ReadLe {
//!     fn read_le(input: &[u8]) -> Self;
//! }
//!
//! repeat!((Ty, Width) in [
//!     (u16, 2),
//!     (u32, 4),
//!     (u64, 8),
//! ] {
//!     impl ReadLe for Ty {
//!         fn read_le(input: &[u8]) -> Self {
//!             Ty::from_le_bytes(input[..Width].try_into().unwrap())
//!         }
//!     }
//! });
//!
//! assert_eq!(u16::read_le(&[0x34, 0x12]), 0x1234);
//! assert_eq!(u32::read_le(&[1, 0, 0, 0]), 1);
//! ```
//!
//! This cannot be written as an ordinary for-loop because `Ty` and `Width` need
//! to be substituted as tokens before the generated code is type-checked.
//!
//! # Whole-body repetition
//!
//! Without splice syntax, [`repeat!`] emits the whole body once per input row:
//!
//! ```rust
//! use macroweave::repeat;
//!
//! trait TypeName {
//!     const NAME: &'static str;
//! }
//!
//! repeat!((Ty, Name) in [
//!     (u8, "u8"),
//!     (u16, "u16"),
//!     (u32, "u32"),
//! ] {
//!     impl TypeName for Ty {
//!         const NAME: &'static str = Name;
//!     }
//! });
//!
//! assert_eq!(<u16 as TypeName>::NAME, "u16");
//! ```
//!
//! # Partial repetition
//!
//! When only part of a surrounding construct should repeat, put that part in
//! `#( ... )*`. A single separator token tree can be written before `*`, such
//! as `#( ... ),*` for comma-separated output:
//!
//! ```rust
//! use macroweave::splice;
//!
//! fn keyword_code(text: &str) -> Option<u8> {
//!     splice!((Pat, Code) in [
//!         ("async", 1u8),
//!         ("await", 2u8),
//!     ] {
//!         match text {
//!             #(Pat => Some(Code)),*,
//!             _ => None,
//!         }
//!     })
//! }
//!
//! assert_eq!(keyword_code("async"), Some(1));
//! assert_eq!(keyword_code("await"), Some(2));
//! assert_eq!(keyword_code("fn"), None);
//! ```
//!
//! When a [`splice!`] body contains `#( ... )*` or `#( ... ),*`, placeholders
//! are substituted only inside the splice body, and the surrounding tokens are
//! emitted once. Surrounding identifiers stay literal, even when they have the
//! same name as a placeholder. If a value should vary, place it in the splice
//! body.
//!
//! `#( ..., )*` and `#( ... ),*` are different: the latter does not produce a
//! trailing comma. This matches delimiter repetition in `macro_rules!`.
//!
//! # Syntax notes
//!
//! - Bind placeholders as bare identifiers, such as `Ty` or `Name`.
//! - Tuple rows bind multiple placeholders, and `_` skips a row value.
//! - Row values can contain one or more Rust tokens. Top-level commas separate rows.
//! - Nested invocations are supported. Use different placeholder names at each level.

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
