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
use syn::braced;
use syn::bracketed;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

#[derive(Clone)]
pub struct Binding {
    pub var: Ident,
    pub tokens: TokenStream,
}

pub struct Template {
    pub rows: Vec<Row>,
    pub template: TokenStream,
}

impl Parse for Template {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut clauses = vec![];
        let mut vars = vec![];

        loop {
            input.parse::<Token![for]>()?;
            let clause = input.parse::<ForClause>()?;
            validate_clause_vars(&clause.vars, &mut vars)?;
            clauses.push(clause.rows);

            if !input.peek(Token![,]) {
                break;
            }

            input.parse::<Token![,]>()?;
            if input.peek(Token![for]) {
                continue;
            }

            if input.peek(syn::token::Brace) {
                break;
            } else {
                return Err(input.error("expected another input clause after comma"));
            }
        }

        let rows = if clauses.len() == 1 {
            clauses
                .pop()
                .expect("input clause list should not be empty")
        } else {
            cartesian_product_rows(clauses)
        };

        let template;
        braced!(template in input);
        let template = template.parse::<TokenStream>()?;

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after template body"));
        }
        Ok(Self { rows, template })
    }
}

pub struct Row {
    pub bindings: Vec<Binding>,
}

impl Row {
    fn empty() -> Self {
        Self { bindings: vec![] }
    }

    fn single(var: &Ident, value: TokenStream) -> Self {
        Self {
            bindings: vec![Binding {
                var: var.clone(),
                tokens: value,
            }],
        }
    }

    fn merge(&self, other: &Self) -> Self {
        let mut bindings = self.bindings.clone();
        bindings.extend(other.bindings.iter().cloned());
        bindings.sort_by(|left, right| left.var.cmp(&right.var));

        Self { bindings }
    }

    fn zip_vars(vars: &TemplateVars, values: Vec<TokenStream>) -> Result<Self> {
        let expected = vars.len();
        let found = values.len();
        if expected != found {
            let mut error = Error::new_spanned(
                TokenStream::from_iter(values.clone()),
                format!(
                    "this row provides {} value{}",
                    found,
                    if found == 1 { "" } else { "s" }
                ),
            );
            error.combine(Error::new_spanned(
                vars,
                format!(
                    "template variables `{}` require {} row value{}",
                    vars.display(),
                    expected,
                    if expected == 1 { "" } else { "s" }
                ),
            ));
            return Err(error);
        }

        let mut bindings = vars
            .idents
            .iter()
            .cloned()
            .zip(values)
            .map(|(var, value)| Binding { var, tokens: value })
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.var.cmp(&right.var));

        Ok(Self { bindings })
    }
}

struct ForClause {
    vars: Vec<Ident>,
    rows: Vec<Row>,
}

impl Parse for ForClause {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vars = input.parse::<TemplateVars>()?;
        let var_idents = vars.idents.clone();
        input.parse::<Token![in]>()?;

        let rows = if input.peek(syn::token::Bracket) {
            let row_values;
            bracketed!(row_values in input);
            parse_rows(&row_values, &vars)?
        } else {
            parse_range_rows(input, &vars)?
        };

        Ok(Self {
            vars: var_idents,
            rows,
        })
    }
}

struct RangeInput {
    start: u64,
    end: u64,
    inclusive: bool,
    format: RangeFormat,
}

impl RangeInput {
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

impl Parse for RangeInput {
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
                "range bounds must be integer, byte, or character literals",
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
                TokenStream::from_iter([start.tokens(), end.tokens()]),
                "range bounds must both be integer literals, both byte literals, or both character literals",
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
                "range bounds must use the same integer suffix",
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
                "range bounds must use the same integer radix",
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
                return Err(Error::new_spanned(tokens, "expected integer range bound"));
            }
        }
    }

    if digits.is_empty() {
        return Err(Error::new_spanned(tokens, "expected integer range bound"));
    }

    let parsed = u64::from_str_radix(&digits, base)
        .map_err(|_| Error::new_spanned(tokens.clone(), "integer range bounds must fit in u64"))?;

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

fn validate_clause_vars(new_vars: &[Ident], existing_vars: &mut Vec<Ident>) -> Result<()> {
    existing_vars.extend_from_slice(new_vars);
    existing_vars.sort();

    for vars in existing_vars.windows(2) {
        let previous = &vars[0];
        let duplicate = &vars[1];
        if previous == duplicate {
            let mut error = Error::new_spanned(
                duplicate,
                format!("the template variable `{duplicate}` duplicates an earlier one"),
            );
            error.combine(Error::new_spanned(
                previous,
                format!("an earlier template variable `{previous}` declared here"),
            ));
            return Err(error);
        }
    }

    Ok(())
}

fn cartesian_product_rows(clauses: Vec<Vec<Row>>) -> Vec<Row> {
    let mut rows = vec![Row::empty()];

    for clause_rows in clauses {
        let mut next_rows = vec![];
        for base in &rows {
            for row in &clause_rows {
                next_rows.push(base.merge(row));
            }
        }
        rows = next_rows;
    }

    rows
}
struct TemplateVars {
    idents: Vec<Ident>,
}

impl ToTokens for TemplateVars {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.idents.clone());
    }
}

impl TemplateVars {
    fn len(&self) -> usize {
        self.idents.len()
    }

    fn validate(&self) -> Result<()> {
        for (index, ident) in self.idents.iter().enumerate() {
            if self.idents[..index]
                .iter()
                .any(|previous| previous == ident)
            {
                return Err(Error::new_spanned(ident, "duplicate template variable"));
            }
        }
        Ok(())
    }

    fn display(&self) -> String {
        match self.idents.as_slice() {
            [ident] => ident.to_string(),
            idents => {
                let names = idents
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({names})")
            }
        }
    }
}

impl Parse for TemplateVars {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::token::Bracket) {
            return Err(input
                .error("multiple template variables must use parentheses, such as `(Ty, Width)`"));
        }

        let idents = if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);

            parse_var_list(&content)?
        } else {
            let ident = input.parse::<Ident>()?;
            vec![ident]
        };

        let vars = Self { idents };
        if vars.idents.is_empty() {
            return Err(input.error("expected at least one template variable"));
        }
        vars.validate()?;
        Ok(vars)
    }
}

fn parse_var_list(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
    Ok(idents.into_iter().collect())
}

fn parse_range_rows(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Vec<Row>> {
    if vars.len() != 1 {
        return Err(Error::new_spanned(
            &vars.idents[0],
            "range inputs require exactly one template variable",
        ));
    }

    let var = &vars.idents[0];
    input
        .parse::<RangeInput>()?
        .values()
        .into_iter()
        .map(|value| Ok(Row::single(var, value)))
        .collect()
}

fn parse_rows(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Vec<Row>> {
    let mut rows = vec![];
    while !input.is_empty() {
        rows.push(parse_row(input, vars)?);
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(rows)
}

fn parse_row(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Row> {
    if vars.len() > 1 {
        if !input.peek(syn::token::Paren) {
            return Err(input.error(
                "rows for multiple template variables must use parentheses, such as `(u16, 2)`",
            ));
        }

        let row;
        parenthesized!(row in input);
        let values = parse_row_values(&row)?;
        if !row.is_empty() {
            return Err(row.error("unexpected tokens in row"));
        }
        return Row::zip_vars(vars, values);
    }

    let value = parse_tokens_until_comma(input)?;

    match vars.len() {
        1 => Ok(Row::single(&vars.idents[0], value)),
        _ => Err(Error::new_spanned(
            &vars.idents[0],
            "plain rows require exactly one template variable",
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
        return Err(input.error("expected row value tokens"));
    }

    Ok(tokens.into_iter().collect())
}
