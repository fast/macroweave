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

use std::env;
use std::fs;
use std::path::PathBuf;

const DOCS_START: &str = "<!-- macroweave-docs-start -->";
const DOCS_END: &str = "<!-- macroweave-docs-end -->";

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readme = manifest_dir.join("README.md");
    let readme = if readme.exists() {
        readme
    } else {
        eprintln!("Warning: README.md not found, skipping documentation extraction");
        return;
    };
    println!("cargo:rerun-if-changed={}", readme.display());

    let readme = fs::read_to_string(readme).unwrap();
    let start = readme
        .find(DOCS_START)
        .unwrap_or_else(|| panic!("missing {DOCS_START} marker in README.md"));
    let end = readme
        .find(DOCS_END)
        .unwrap_or_else(|| panic!("missing {DOCS_END} marker in README.md"));
    assert!(
        start < end,
        "{DOCS_START} must appear before {DOCS_END} in README.md"
    );
    let crate_docs = readme[start + DOCS_START.len()..end].trim();

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("crate-docs.md"), crate_docs).unwrap();
}
