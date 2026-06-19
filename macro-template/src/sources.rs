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

use proc_macro2::Ident;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use syn::Error;
use syn::Lit;
use syn::Result;
use syn::Token;
use syn::bracketed;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

use crate::expands::Replacement;

pub struct SourceRow {
    pub replacements: Vec<Replacement>,
}

impl SourceRow {
    fn empty() -> Self {
        Self {
            replacements: vec![],
        }
    }

    fn single(placeholder: &Ident, value: TokenStream) -> Self {
        Self {
            replacements: vec![Replacement::new(placeholder.clone(), value)],
        }
    }

    fn merge(&self, other: &Self) -> Self {
        let mut replacements = self.replacements.clone();
        replacements.extend(other.replacements.iter().cloned());
        replacements.sort_by(|left, right| left.placeholder().cmp(right.placeholder()));

        Self { replacements }
    }

    fn zip_placeholders(placeholders: &Placeholders, values: Vec<TokenStream>) -> Result<Self> {
        let expected = placeholders.len();
        let found = values.len();
        let span_tokens = join_tokens(values.iter().cloned());
        if expected != found {
            return Err(Error::new_spanned(
                &span_tokens,
                format!("expected {expected} replacement values, found {found}"),
            ));
        }

        let mut replacements = placeholders
            .idents
            .iter()
            .cloned()
            .zip(values)
            .map(|(placeholder, value)| Replacement::new(placeholder, value))
            .collect::<Vec<_>>();
        replacements.sort_by(|left, right| left.placeholder().cmp(right.placeholder()));

        Ok(Self { replacements })
    }
}

pub struct Sources {
    pub rows: Vec<SourceRow>,
}

impl Parse for Sources {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut sources = vec![];
        let mut placeholders = vec![];

        loop {
            input.parse::<Token![for]>()?;
            let source = input.parse::<Source>()?;
            validate_source_placeholders(&source.placeholders, &mut placeholders)?;
            sources.push(source.rows);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.peek(Token![for]) {
                    continue;
                }
                if input.peek(syn::token::Brace) {
                    break;
                } else {
                    return Err(input.error("expected another source after comma"));
                }
            } else {
                break;
            }
        }

        let rows = if sources.len() == 1 {
            sources.pop().expect("source list should not be empty")
        } else {
            cartesian_product_rows(sources)
        };

        Ok(Self { rows })
    }
}

struct Source {
    placeholders: Vec<Ident>,
    rows: Vec<SourceRow>,
}

impl Parse for Source {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let placeholders = input.parse::<Placeholders>()?;
        let placeholder_idents = placeholders.idents.clone();
        input.parse::<Token![in]>()?;

        let rows = if input.peek(syn::token::Bracket) {
            let source;
            bracketed!(source in input);
            let rows = parse_source_rows(&source, &placeholders)?;
            if rows.is_empty() {
                return Err(source.error("expected at least one template row"));
            }
            rows
        } else {
            parse_range_source_rows(input, &placeholders)?
        };

        Ok(Self {
            placeholders: placeholder_idents,
            rows,
        })
    }
}

struct RangeSource {
    start: u64,
    end: u64,
    inclusive: bool,
    format: RangeFormat,
}

impl RangeSource {
    fn values(&self) -> Vec<TokenStream> {
        let mut values = vec![];
        if self.start > self.end || (!self.inclusive && self.start == self.end) {
            return values;
        }

        let iter: Box<dyn Iterator<Item = u64>> = if self.inclusive {
            Box::new(self.start..=self.end)
        } else {
            Box::new(self.start..self.end)
        };

        for value in iter {
            if let Some(tokens) = self.format.value_tokens(value) {
                values.push(tokens);
            }
        }

        values
    }
}

impl Parse for RangeSource {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let start = input.parse::<RangeBound>()?;
        let inclusive = if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            true
        } else {
            input.parse::<Token![..]>()?;
            false
        };
        let end = input.parse::<RangeBound>()?;

        let format = RangeFormat::from_bounds(&start, &end)?;

        Ok(Self {
            start: start.value,
            end: end.value,
            inclusive,
            format,
        })
    }
}

struct RangeBound {
    value: u64,
    format: RangeFormat,
    tokens: TokenStream,
}

impl RangeBound {
    fn tokens(&self) -> TokenStream {
        self.tokens.clone()
    }
}

impl Parse for RangeBound {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let literal = input.parse::<Lit>()?;
        let tokens = literal.clone().into_token_stream();

        match literal {
            Lit::Int(value) => parse_integer_bound(value),
            Lit::Byte(value) => Ok(Self {
                value: u64::from(value.value()),
                format: RangeFormat::Byte,
                tokens,
            }),
            Lit::Char(value) => Ok(Self {
                value: u64::from(u32::from(value.value())),
                format: RangeFormat::Character,
                tokens,
            }),
            _ => Err(Error::new_spanned(
                tokens,
                "range source bounds must be integer, byte, or character literals",
            )),
        }
    }
}

enum RangeFormat {
    Integer(IntegerFormat),
    Byte,
    Character,
}

struct IntegerFormat {
    suffix: String,
    padding_width: usize,
    radix: RangeRadix,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeRadix {
    Binary,
    Octal,
    Decimal,
    LowerHex,
    UpperHex,
}

impl RangeFormat {
    fn from_bounds(start: &RangeBound, end: &RangeBound) -> Result<Self> {
        match (&start.format, &end.format) {
            (Self::Integer(start_format), Self::Integer(end_format)) => {
                IntegerFormat::from_bounds(start_format, end_format, &end.tokens).map(Self::Integer)
            }
            (Self::Byte, Self::Byte) => Ok(Self::Byte),
            (Self::Character, Self::Character) => Ok(Self::Character),
            _ => Err(Error::new_spanned(
                join_tokens([start.tokens(), end.tokens()]),
                "range source bounds must both be integer literals, both byte literals, or both character literals",
            )),
        }
    }

    fn value_tokens(&self, value: u64) -> Option<TokenStream> {
        match self {
            Self::Integer(format) => Some(format.value_tokens(value)),
            Self::Byte => Some(Literal::u64_unsuffixed(value).into_token_stream()),
            Self::Character => u32::try_from(value)
                .ok()
                .and_then(char::from_u32)
                .map(|value| Literal::character(value).into_token_stream()),
        }
    }
}

impl IntegerFormat {
    fn from_bounds(start: &Self, end: &Self, end_tokens: &TokenStream) -> Result<Self> {
        let suffix = if start.suffix.is_empty() {
            end.suffix.clone()
        } else if end.suffix.is_empty() || start.suffix == end.suffix {
            start.suffix.clone()
        } else {
            return Err(Error::new_spanned(
                end_tokens,
                "range source bounds must use the same integer suffix",
            ));
        };

        let radix = if start.radix == end.radix {
            start.radix
        } else if matches!(
            (start.radix, end.radix),
            (RangeRadix::LowerHex, RangeRadix::UpperHex)
                | (RangeRadix::UpperHex, RangeRadix::LowerHex)
        ) {
            RangeRadix::UpperHex
        } else {
            return Err(Error::new_spanned(
                end_tokens,
                "range source bounds must use the same integer radix",
            ));
        };

        Ok(Self {
            suffix,
            padding_width: start.padding_width.min(end.padding_width),
            radix,
        })
    }

    fn value_tokens(&self, value: u64) -> TokenStream {
        let width = self.padding_width;
        let repr = match self.radix {
            RangeRadix::Binary => {
                format!("0b{:0width$b}{}", value, self.suffix)
            }
            RangeRadix::Octal => format!("0o{:0width$o}{}", value, self.suffix),
            RangeRadix::Decimal => format!("{:0width$}{}", value, self.suffix),
            RangeRadix::LowerHex => {
                format!("0x{:0width$x}{}", value, self.suffix)
            }
            RangeRadix::UpperHex => {
                format!("0x{:0width$X}{}", value, self.suffix)
            }
        };

        repr.parse().expect("generated range literal should parse")
    }
}

fn parse_integer_bound(value: syn::LitInt) -> Result<RangeBound> {
    let tokens = value.clone().into_token_stream();
    let repr = value.to_string();
    let suffix = value.suffix().to_owned();
    let literal = if suffix.is_empty() {
        repr.as_str()
    } else {
        &repr[..repr.len() - suffix.len()]
    };

    let (mut radix, base, digits_start) = if literal.starts_with("0b") {
        (RangeRadix::Binary, 2, 2)
    } else if literal.starts_with("0o") {
        (RangeRadix::Octal, 8, 2)
    } else if literal.starts_with("0x") {
        (RangeRadix::LowerHex, 16, 2)
    } else {
        (RangeRadix::Decimal, 10, 0)
    };

    let body = &literal[digits_start..];
    let mut digits = String::new();

    for ch in body.chars() {
        match ch {
            '_' => {}
            '0'..='9' => digits.push(ch),
            'A'..='F' if radix == RangeRadix::LowerHex => {
                digits.push(ch);
                radix = RangeRadix::UpperHex;
            }
            'a'..='f' | 'A'..='F' if base == 16 => digits.push(ch),
            _ => {
                return Err(Error::new_spanned(
                    tokens,
                    "expected integer range source bound",
                ));
            }
        }
    }

    if digits.is_empty() {
        return Err(Error::new_spanned(
            tokens,
            "expected integer range source bound",
        ));
    }

    let parsed = u64::from_str_radix(&digits, base).map_err(|_| {
        Error::new_spanned(
            tokens.clone(),
            "integer range source bounds must fit in u64",
        )
    })?;

    Ok(RangeBound {
        value: parsed,
        format: RangeFormat::Integer(IntegerFormat {
            suffix,
            padding_width: digits.len(),
            radix,
        }),
        tokens,
    })
}

fn validate_source_placeholders(
    new_placeholders: &[Ident],
    existing_placeholders: &mut Vec<Ident>,
) -> Result<()> {
    existing_placeholders.extend_from_slice(new_placeholders);
    existing_placeholders.sort();

    for placeholders in existing_placeholders.windows(2) {
        let previous = &placeholders[0];
        let duplicate = &placeholders[1];
        if previous == duplicate {
            let mut error = Error::new_spanned(
                duplicate,
                format!("the placeholder `{duplicate}` duplicates an earlier one"),
            );
            error.combine(Error::new_spanned(
                previous,
                format!("an earlier placeholder `{previous}` declared here"),
            ));
            return Err(error);
        }
    }

    Ok(())
}

fn cartesian_product_rows(sources: Vec<Vec<SourceRow>>) -> Vec<SourceRow> {
    let mut rows = vec![SourceRow::empty()];

    for source_rows in sources {
        let mut next_rows = vec![];
        for base in &rows {
            for row in &source_rows {
                next_rows.push(base.merge(row));
            }
        }
        rows = next_rows;
    }

    rows
}
struct Placeholders {
    idents: Vec<Ident>,
}

impl Placeholders {
    fn len(&self) -> usize {
        self.idents.len()
    }

    fn validate(&self) -> Result<()> {
        for (index, ident) in self.idents.iter().enumerate() {
            if self.idents[..index]
                .iter()
                .any(|previous| previous == ident)
            {
                return Err(Error::new_spanned(ident, "duplicate template placeholder"));
            }
        }
        Ok(())
    }
}

impl Parse for Placeholders {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::token::Bracket) {
            return Err(input.error(
                "multiple template placeholders must use parentheses, such as `(Ty, Width)`",
            ));
        }

        let idents = if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            parse_placeholder_list(&content)?
        } else {
            vec![input.parse()?]
        };

        let placeholders = Self { idents };
        if placeholders.idents.is_empty() {
            return Err(input.error("expected at least one template placeholder"));
        }
        placeholders.validate()?;
        Ok(placeholders)
    }
}

fn parse_placeholder_list(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
    Ok(idents.into_iter().collect())
}

fn parse_range_source_rows(
    input: ParseStream<'_>,
    placeholders: &Placeholders,
) -> Result<Vec<SourceRow>> {
    if placeholders.len() != 1 {
        return Err(Error::new_spanned(
            &placeholders.idents[0],
            "range sources require exactly one template placeholder",
        ));
    }

    let placeholder = &placeholders.idents[0];
    input
        .parse::<RangeSource>()?
        .values()
        .into_iter()
        .map(|value| Ok(SourceRow::single(placeholder, value)))
        .collect()
}

fn parse_source_rows(
    input: ParseStream<'_>,
    placeholders: &Placeholders,
) -> Result<Vec<SourceRow>> {
    let mut rows = vec![];
    while !input.is_empty() {
        rows.push(parse_source_row(input, placeholders)?);
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(rows)
}

fn parse_source_row(input: ParseStream<'_>, placeholders: &Placeholders) -> Result<SourceRow> {
    if placeholders.len() > 1 {
        if !input.peek(syn::token::Paren) {
            return Err(input
                .error("multi-placeholder source rows must use parentheses, such as `(u16, 2)`"));
        }

        let row;
        parenthesized!(row in input);
        let values = parse_row_values(&row)?;
        if !row.is_empty() {
            return Err(row.error("unexpected tokens in template row"));
        }
        return SourceRow::zip_placeholders(placeholders, values);
    }

    let value = parse_tokens_until_comma(input)?;

    match placeholders.len() {
        1 => Ok(SourceRow::single(&placeholders.idents[0], value)),
        _ => Err(Error::new_spanned(
            &placeholders.idents[0],
            "plain rows require exactly one placeholder",
        )),
    }
}

fn parse_row_values(input: ParseStream<'_>) -> Result<Vec<TokenStream>> {
    let mut values = vec![];
    while !input.is_empty() {
        values.push(parse_tokens_until_comma(input)?);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(values)
}

fn parse_tokens_until_comma(input: ParseStream<'_>) -> Result<TokenStream> {
    let mut tokens = vec![];
    while !input.is_empty() {
        if input.peek(Token![,]) {
            break;
        }
        tokens.push(input.parse::<TokenTree>()?);
    }

    if tokens.is_empty() {
        return Err(input.error("expected replacement tokens"));
    }

    Ok(tokens.into_iter().collect())
}

fn join_tokens(tokens: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    tokens
        .into_iter()
        .map(TokenStream::into_token_stream)
        .collect()
}
