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

//! [`template!`] is a procedural macro that generates repeated Rust code in multiple places with
//! table-driven sources.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

/// Expands an item, block, or statement template from one or more sources.
#[proc_macro]
pub fn template(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    unimplemented!("template macro expansion")
}
