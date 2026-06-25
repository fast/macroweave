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

#![allow(dead_code)]
#![allow(clippy::vec_init_then_push)]

use macroweave::repeat;
use macroweave::splice;

struct TypeA;
struct TypeB;
struct InputA;
struct InputB;

trait TypeTag {
    const TAG: &'static str;
}

repeat!(T in [TypeA, TypeB] {
    impl TypeTag for T {
        const TAG: &'static str = stringify!(T);
    }
});

#[test]
fn repeat_expands_items() {
    assert_eq!(TypeA::TAG, "TypeA");
    assert_eq!(TypeB::TAG, "TypeB");
}

#[test]
fn repeat_expands_statements() {
    let mut values = vec![];
    let one = 1usize;
    let two = 2usize;
    let three = 3usize;

    repeat!(value in [one, two, three] {
        values.push(value);
    });

    assert_eq!(values, [1, 2, 3]);
}

#[test]
fn repeat_accepts_trailing_comma_in_input_list() {
    let mut values = vec![];
    let one = 1usize;
    let two = 2usize;

    repeat!(value in [one, two,] {
        values.push(value);
    });

    assert_eq!(values, [1, 2]);
}

#[test]
fn repeat_accepts_multi_token_input_items() {
    let mut count = 0usize;

    repeat!(T in [
        Option<u8>,
        Result<u8, &'static str>,
    ] {
        let _ = size_of::<T>();
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn repeat_leaves_unmatched_identifiers_unchanged() {
    let mut values = vec![];
    let source = 1usize;

    repeat!(placeholder in [source] {
        let value = 2usize;
        values.push(value);
        values.push(placeholder);
    });

    assert_eq!(values, [2, 1]);
}

#[test]
fn repeat_expands_tuple_bindings() {
    let mut total = 0usize;
    let one = 1usize;
    let two = 2usize;

    repeat!((name, value) in [(first, one), (second, two)] {
        let name = value;
        total += name;
    });

    assert_eq!(total, 3);
}

#[test]
fn repeat_accepts_multi_token_tuple_row_values() {
    let mut count = 0usize;

    repeat!((T, value) in [
        (Option<u8>, None::<u8>),
        (
            Result<u8, &'static str>,
            Ok::<u8, &'static str>(1u8),
        ),
    ] {
        let _: T = value;
        count += 1;
    });

    assert_eq!(count, 2);
}

#[test]
fn repeat_accepts_single_value_tuple_rows() {
    let mut names = vec![];

    repeat!((T,) in [(TypeA,), (TypeB,)] {
        names.push(stringify!(T));
    });

    assert_eq!(names, ["TypeA", "TypeB"]);
}

#[test]
fn repeat_accepts_trailing_comma_after_tuple_rows() {
    let mut values = vec![];
    let one = 1usize;
    let two = 2usize;

    repeat!((name, value) in [(first, one), (second, two),] {
        let name = value;
        values.push(name);
    });

    assert_eq!(values, [1, 2]);
}

#[test]
fn repeat_ignores_underscore_bindings() {
    let mut names = vec![];

    repeat!((T, _) in [(TypeA, one), (TypeB, two)] {
        names.push(stringify!(T));
    });

    assert_eq!(names, ["TypeA", "TypeB"]);
}

#[test]
fn repeat_expands_nested_repeats() {
    let mut names = vec![];

    repeat!(T in [TypeA, TypeB] {
        repeat!(U in [InputA, InputB] {
            names.push(stringify!(T U));
        });
    });

    assert_eq!(
        names,
        [
            "TypeA InputA",
            "TypeA InputB",
            "TypeB InputA",
            "TypeB InputB"
        ]
    );
}

#[test]
fn repeat_works_with_renamed_nested_macros_when_names_are_distinct() {
    use macroweave::repeat as r;

    let mut names = vec![];

    repeat!(T in [TypeA, TypeB] {
        r!(U in [T] {
            names.push(stringify!(T U));
        });
    });

    assert_eq!(names, ["TypeA TypeA", "TypeB TypeB"]);
}

#[test]
fn repeat_preserves_hash_paren_non_repetition_for_downstream_macros() {
    macro_rules! stringify_tokens {
        ($($tokens:tt)*) => {
            stringify!($($tokens)*)
        };
    }

    repeat!(value in [source] {
        let tokens = stringify_tokens! { #( value )+ };
    });

    assert_eq!(tokens, "# (source) +");
}

enum MyType {
    CaseA(u8),
    CaseB(u8),
    Other(u8),
}

fn describe(value: MyType) -> String {
    splice!(C in [CaseA, CaseB] {
        match value {
            #( MyType::C(value) => value.to_string(), )*
            MyType::Other(_) => String::new(),
        }
    })
}

#[test]
fn splice_expands_match_arms() {
    assert_eq!(describe(MyType::CaseA(1)), "1");
    assert_eq!(describe(MyType::CaseB(2)), "2");
    assert_eq!(describe(MyType::Other(3)), "");
}

splice!(Variant in [First, Second] {
    #[derive(Debug, PartialEq, Eq)]
    enum SpliceEnum {
        #( Variant ),*,
        Other,
    }
});

#[test]
fn splice_expands_enum_variants() {
    assert_eq!(format!("{:?}", SpliceEnum::First), "First");
    assert_eq!(format!("{:?}", SpliceEnum::Second), "Second");
    assert_eq!(format!("{:?}", SpliceEnum::Other), "Other");
}

#[test]
fn splice_expands_multiple_fragments_from_the_same_list() {
    splice!(Variant in [First, Second] {
        let names = [#( stringify!(Variant) ),*];
        let values = [#( SpliceEnum::Variant ),*];
    });

    assert_eq!(names, ["First", "Second"]);
    assert_eq!(values, [SpliceEnum::First, SpliceEnum::Second]);
}

#[test]
fn splice_expands_without_separator() {
    let mut values = vec![];
    let one = 1usize;
    let two = 2usize;
    let three = 3usize;

    splice!(value in [one, two, three] {
        #( values.push(value); )*
    });

    assert_eq!(values, [1, 2, 3]);
}

#[test]
fn splice_accepts_multi_token_input_items() {
    splice!(value in [1 + 1, 2 + 2] {
        let values = [#( value ),*];
    });

    assert_eq!(values, [2, 4]);
}

#[test]
fn splice_accepts_token_tree_separator() {
    macro_rules! stringify_tokens {
        ($($tokens:tt)*) => {
            stringify!($($tokens)*)
        };
    }

    splice!(word in [first, second] {
        let tokens = stringify_tokens! { #( word )(separator)* };
    });

    assert_eq!(tokens, "first(separator) second");
}
