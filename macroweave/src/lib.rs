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

#![doc = include_str!(concat!(env!("OUT_DIR"), "/crate-docs.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

mod expand;
mod parse;

/// Repeats the whole body once for each input row.
///
/// See the [crate-level documentation](self) for syntax details, tuple bindings, nesting behavior,
/// and input restrictions.
#[proc_macro]
pub fn repeat(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::repeat(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Emits the body once and expands each `#( ... )*` fragment from the input rows.
///
/// See the [crate-level documentation](self) for repetition syntax, scope rules, and nesting
/// behavior.
#[proc_macro]
pub fn splice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::splice(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
