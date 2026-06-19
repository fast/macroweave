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

use macro_template::template;

template! {
    for N in 0..3 {
        const _: usize = N;
    }
}

trait TypeTag {
    const TAG: u8;
}

template! {
    for (Ty, Tag) in [
        (bool, 1),
        (usize, 2),
    ] {
        impl TypeTag for Ty {
            const TAG: u8 = Tag;
        }
    }
}

#[test]
fn expands_item_templates() {
    assert_eq!(<bool as TypeTag>::TAG, 1);
    assert_eq!(<usize as TypeTag>::TAG, 2);
}

#[test]
fn expands_statement_templates() {
    let mut total = 0usize;

    template! {
        for (Name, Value) in [
            (first, 1usize),
            (second, 2usize),
        ] {
            let Name = Value;
            total += Name;
        }
    }

    assert_eq!(total, 3);
}

#[test]
fn accepts_trailing_comma_after_single_input_clause() {
    template! {
        for N in [1usize, 2usize],
        {
            const _: usize = N;
        }
    }
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn preserves_grouped_commas_in_single_variable_rows() {
    let mut pairs = vec![];

    template! {
        for Pair in [(1usize, 2usize), (3usize, 4usize)] {
            pairs.push(Pair);
        }
    }

    assert_eq!(pairs, [(1, 2), (3, 4)]);
}

#[test]
fn preserves_grouped_commas_in_tuple_row_values() {
    let mut pairs = vec![];

    template! {
        for (Name, Pair) in [
            (first, (1usize, 2usize)),
            (second, (3usize, 4usize)),
        ] {
            let Name = Pair;
            pairs.push(Name);
        }
    }

    assert_eq!(pairs, [(1, 2), (3, 4)]);
}

#[test]
fn expands_splice_block_without_repeating_surrounding_tokens() {
    template! {
        for N in 0..=2 {
            let values = [100usize, @splice { N, } 200usize];
        }
    }

    assert_eq!(values, [100, 0, 1, 2, 200]);
}

template! {
    for Variant in [First, Second] {
        #[derive(Debug, PartialEq, Eq)]
        enum SpliceBlockEnum {
            @splice { Variant, }
            Other,
        }
    }
}

#[test]
fn expands_splice_block_in_item_groups() {
    assert_eq!(format!("{:?}", SpliceBlockEnum::First), "First");
    assert_eq!(format!("{:?}", SpliceBlockEnum::Second), "Second");
    assert_eq!(format!("{:?}", SpliceBlockEnum::Other), "Other");
}

template! {
    for (Name, Variant) in [
        (IgnoredOne, Alpha),
        (IgnoredTwo, Beta),
    ] {
        #[derive(Debug, PartialEq, Eq)]
        enum Name {
            @splice { Variant, }
        }
    }
}

#[test]
fn preserves_outer_variable_tokens_in_splice_templates() {
    assert_eq!(format!("{:?}", Name::Alpha), "Alpha");
    assert_eq!(format!("{:?}", Name::Beta), "Beta");
}

#[test]
fn preserves_hash_paren_repetition_for_downstream_macros() {
    macro_rules! stringify_tokens {
        ($($tokens:tt)*) => {
            stringify!($($tokens)*)
        };
    }

    template! {
        for N in [0] {
            let tokens = stringify_tokens! { #(N)* };
        }
    }

    assert_eq!(tokens, "# (0) *");
}

#[test]
fn preserves_at_ident_for_downstream_macros() {
    macro_rules! stringify_tokens {
        ($($tokens:tt)*) => {
            stringify!($($tokens)*)
        };
    }

    template! {
        for N in [0] {
            let tokens = stringify_tokens! { @N };
        }
    }

    assert_eq!(tokens, "@ 0");
}

#[test]
fn preserves_bare_at_brace_for_downstream_macros() {
    macro_rules! stringify_tokens {
        ($($tokens:tt)*) => {
            stringify!($($tokens)*)
        };
    }

    template! {
        for N in [0] {
            let tokens = stringify_tokens! { @{ N } };
        }
    }

    assert_eq!(tokens, "@ { 0 }");
}

#[test]
fn expands_match_arms_from_splice_block() {
    fn parse_keyword(text: &str) -> Option<u8> {
        template! {
            for (Pat, Value) in [
                ("async", 1u8),
                ("await", 2u8),
            ] {
                match text {
                    @splice { Pat => Some(Value), }
                    _ => None,
                }
            }
        }
    }

    assert_eq!(parse_keyword("async"), Some(1));
    assert_eq!(parse_keyword("await"), Some(2));
    assert_eq!(parse_keyword("fn"), None);
}

#[test]
fn treats_fat_arrow_as_plain_row_tokens() {
    fn classify(value: u8) -> Option<&'static str> {
        template! {
            for Arm in [0 => Some("zero"), 1 => Some("one")] {
                match value {
                    @splice { Arm, }
                    _ => None,
                }
            }
        }
    }

    assert_eq!(classify(0), Some("zero"));
    assert_eq!(classify(1), Some("one"));
    assert_eq!(classify(2), None);
}

#[test]
fn expands_integer_range_input_for_tuple_fields() {
    let tuple = (1usize, 2usize, 3usize);
    let mut sum = 0usize;

    template! {
        for N in 0..=2 {
            sum += tuple.N;
        }
    }

    assert_eq!(sum, 6);
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn expands_character_range_input() {
    let mut chars = vec![];

    template! {
        for C in 'a'..='c' {
            chars.push(C);
        }
    }

    assert_eq!(chars, ['a', 'b', 'c']);
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn expands_byte_range_input() {
    let mut bytes = Vec::<u8>::new();

    template! {
        for B in b'x'..=b'z' {
            bytes.push(B);
        }
    }

    assert_eq!(bytes, b"xyz");
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn preserves_integer_range_radix_and_padding() {
    let mut lower_hex = vec![];
    template! {
        for N in 0x08..=0x0b {
            lower_hex.push(stringify!(N));
        }
    }
    assert_eq!(lower_hex, ["0x08", "0x09", "0x0a", "0x0b"]);

    let mut upper_hex = vec![];
    template! {
        for N in 0x08..=0x0B {
            upper_hex.push(stringify!(N));
        }
    }
    assert_eq!(upper_hex, ["0x08", "0x09", "0x0A", "0x0B"]);

    let mut binary = vec![];
    template! {
        for N in 0b001..=0b011 {
            binary.push(stringify!(N));
        }
    }
    assert_eq!(binary, ["0b001", "0b010", "0b011"]);

    let mut octal = vec![];
    template! {
        for N in 0o06..=0o10 {
            octal.push(stringify!(N));
        }
    }
    assert_eq!(octal, ["0o06", "0o07", "0o10"]);

    let mut decimal = vec![];
    template! {
        for N in 098..=100 {
            decimal.push(stringify!(N));
        }
    }
    assert_eq!(decimal, ["098", "099", "100"]);

    let mut padded_start = vec![];
    template! {
        for N in 00..=03 {
            padded_start.push(stringify!(N));
        }
    }
    assert_eq!(padded_start, ["00", "01", "02", "03"]);

    let mut padded_end = vec![];
    template! {
        for N in 008..=010 {
            padded_end.push(stringify!(N));
        }
    }
    assert_eq!(padded_end, ["008", "009", "010"]);

    let mut padded_hex_start = vec![];
    template! {
        for N in 0x0008..=0x000A {
            padded_hex_start.push(stringify!(N));
        }
    }
    assert_eq!(padded_hex_start, ["0x0008", "0x0009", "0x000A"]);

    let mut one_sided_padding = vec![];
    template! {
        for N in 00..=3 {
            one_sided_padding.push(stringify!(N));
        }
    }
    assert_eq!(one_sided_padding, ["0", "1", "2", "3"]);

    let mut one_sided_hex_padding = vec![];
    template! {
        for N in 0x0008..=0x0A {
            one_sided_hex_padding.push(stringify!(N));
        }
    }
    assert_eq!(one_sided_hex_padding, ["0x08", "0x09", "0x0A"]);
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn preserves_integer_range_suffix() {
    let mut values = Vec::<u16>::new();

    template! {
        for N in 0u16..=2u16 {
            values.push(N);
        }
    }

    assert_eq!(values, [0, 1, 2]);
}

trait Kernel<T> {
    fn run(input: T) -> T;
}

struct Cpu;
struct Gpu;

template! {
    for Backend in [Cpu, Gpu],
    for Ty in [f32, f64],
    {
        impl Kernel<Ty> for Backend {
            fn run(input: Ty) -> Ty {
                input
            }
        }
    }
}

#[test]
fn expands_cartesian_product_items() {
    assert_eq!(<Cpu as Kernel<f32>>::run(1.0), 1.0);
    assert_eq!(<Cpu as Kernel<f64>>::run(1.0), 1.0);
    assert_eq!(<Gpu as Kernel<f32>>::run(1.0), 1.0);
    assert_eq!(<Gpu as Kernel<f64>>::run(1.0), 1.0);
}

struct DatabasesView;
struct SchemasView;

trait SystemView<T> {
    const NAME: &'static str;
}

template! {
    for (Variant, View) in [
        (Databases, DatabasesView),
        (Schemas, SchemasView),
    ],
    for Ty in [u8, u16] {
        impl SystemView<Ty> for View {
            const NAME: &'static str = stringify!(Variant);
        }
    }
}

#[test]
fn combines_tuple_rows_and_list_inputs() {
    assert_eq!(<DatabasesView as SystemView<u8>>::NAME, "Databases");
    assert_eq!(<DatabasesView as SystemView<u16>>::NAME, "Databases");
    assert_eq!(<SchemasView as SystemView<u8>>::NAME, "Schemas");
    assert_eq!(<SchemasView as SystemView<u16>>::NAME, "Schemas");
}

struct Login;
struct Data;

trait MessageCode<const VERSION: usize> {
    const CODE: u16;
}

template! {
    for (Message, BaseCode) in [
        (Login, 0x1000u16),
        (Data, 0x2000u16),
    ],
    for Version in 1..=2 {
        impl MessageCode<Version> for Message {
            const CODE: u16 = BaseCode + Version;
        }
    }
}

#[test]
fn combines_tuple_rows_and_range_inputs() {
    assert_eq!(<Login as MessageCode<1>>::CODE, 0x1001);
    assert_eq!(<Login as MessageCode<2>>::CODE, 0x1002);
    assert_eq!(<Data as MessageCode<1>>::CODE, 0x2001);
    assert_eq!(<Data as MessageCode<2>>::CODE, 0x2002);
}

#[test]
fn expands_splice_block_over_cartesian_rows() {
    template! {
        for Left in [1usize, 2usize],
        for Right in [10usize, 20usize] {
            const PAIRS: &[(usize, usize)] = &[
                @splice { (Left, Right), }
            ];
        }
    }

    assert_eq!(PAIRS, [(1, 10), (1, 20), (2, 10), (2, 20)]);
}

#[test]
#[allow(clippy::vec_init_then_push)]
fn expands_statement_cartesian_product() {
    let mut values = vec![];

    template! {
        for Prefix in ["read", "write"],
        for Code in 200..=201 {
            values.push((Prefix, Code));
        }
    }

    assert_eq!(
        values,
        [("read", 200), ("read", 201), ("write", 200), ("write", 201),]
    );
}
