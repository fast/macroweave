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
    pub table: Table,
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
            clauses.push(clause.table);

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

        let table = table_join(clauses);

        let template;
        braced!(template in input);
        let template = template.parse::<TokenStream>()?;

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after template body"));
        }
        Ok(Self { table, template })
    }
}

pub struct Table {
    bindings: Vec<Binding>,
    row_count: usize,
    column_count: usize,
}

pub struct RowsIter<'a> {
    table: &'a Table,
    row: usize,
}

impl Table {
    fn empty(column_count: usize) -> Self {
        Self {
            bindings: vec![],
            row_count: 0,
            column_count,
        }
    }

    fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    pub fn rows(&self) -> RowsIter<'_> {
        self.assert_invariant();

        RowsIter {
            table: self,
            row: 0,
        }
    }

    fn add_single_row(&mut self, var: &Ident, value: TokenStream) {
        debug_assert_eq!(self.column_count, 1);

        self.bindings.push(Binding {
            var: var.clone(),
            tokens: value,
        });
        self.row_count += 1;
        self.assert_invariant();
    }

    fn add_row(&mut self, vars: &TemplateVars, values: Vec<TokenStream>) -> Result<()> {
        debug_assert_eq!(self.column_count, vars.len());

        let expected = self.column_count;
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

        let row_start = self.bindings.len();
        self.bindings.extend(
            vars.idents
                .iter()
                .cloned()
                .zip(values)
                .map(|(var, value)| Binding { var, tokens: value }),
        );
        self.bindings[row_start..].sort_by(|left, right| left.var.cmp(&right.var));
        self.row_count += 1;
        self.assert_invariant();

        Ok(())
    }

    fn join(&self, other: &Self) -> Self {
        self.assert_invariant();
        other.assert_invariant();

        let column_count = self.column_count + other.column_count;
        let row_count = self.row_count * other.row_count;
        let mut bindings = Vec::with_capacity(row_count * column_count);

        for left in self.rows() {
            for right in other.rows() {
                let row_start = bindings.len();
                bindings.extend(left.iter().cloned());
                bindings.extend(right.iter().cloned());
                bindings[row_start..].sort_by(|left, right| left.var.cmp(&right.var));
            }
        }

        let table = Self {
            bindings,
            row_count,
            column_count,
        };
        table.assert_invariant();
        table
    }

    fn row(&self, row: usize) -> &[Binding] {
        debug_assert!(row < self.row_count);

        let start = row * self.column_count;
        let end = start + self.column_count;
        &self.bindings[start..end]
    }

    fn assert_invariant(&self) {
        debug_assert_eq!(self.bindings.len(), self.row_count * self.column_count);
    }
}

impl<'a> Iterator for RowsIter<'a> {
    type Item = &'a [Binding];

    fn next(&mut self) -> Option<Self::Item> {
        if self.row == self.table.row_count {
            return None;
        }

        let row = self.table.row(self.row);
        self.row += 1;
        Some(row)
    }
}

struct ForClause {
    vars: Vec<Ident>,
    table: Table,
}

impl Parse for ForClause {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vars = input.parse::<TemplateVars>()?;
        let var_idents = vars.idents.clone();
        input.parse::<Token![in]>()?;

        let table = if input.peek(syn::token::Bracket) {
            let row_values;
            let bracket_token = bracketed!(row_values in input);
            let table = parse_rows(&row_values, &vars)?;
            if table.is_empty() {
                return Err(Error::new(
                    bracket_token.span.join(),
                    "input list must contain at least one row",
                ));
            }
            table
        } else {
            parse_range_rows(input, &vars)?
        };

        Ok(Self {
            vars: var_idents,
            table,
        })
    }
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

fn table_join(clauses: Vec<Table>) -> Table {
    let mut clauses = clauses.into_iter();
    let mut table = clauses
        .next()
        .expect("template should have at least one input clause");

    for clause in clauses {
        table = table.join(&clause);
    }

    table
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
        let idents = if input.peek(syn::token::Paren) {
            let content;
            let paren_token = parenthesized!(content in input);
            let idents = parse_var_list(&content)?;
            if idents.is_empty() {
                return Err(Error::new(
                    paren_token.span.join(),
                    "expected at least one template variable",
                ));
            }
            idents
        } else if let Ok(ident) = input.parse::<Ident>() {
            vec![ident]
        } else {
            return Err(input
                .error("multiple template variables must use parentheses, such as `(Ty, Width)`"));
        };

        let vars = Self { idents };
        vars.validate()?;
        Ok(vars)
    }
}

fn parse_var_list(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
    Ok(idents.into_iter().collect())
}

fn parse_range_rows(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Table> {
    if vars.len() != 1 {
        return Err(Error::new_spanned(
            &vars.idents[0],
            "range inputs require exactly one template variable",
        ));
    }

    let var = &vars.idents[0];
    let range = input.parse::<RangeInput>()?;
    let values = range.values();
    if values.is_empty() {
        return Err(Error::new_spanned(
            range.tokens,
            "range input must contain at least one value",
        ));
    }

    let mut table = Table::empty(vars.len());
    for value in values {
        table.add_single_row(var, value);
    }
    Ok(table)
}

fn parse_rows(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Table> {
    let mut table = Table::empty(vars.len());
    while !input.is_empty() {
        let values = parse_row(input, vars)?;
        table.add_row(vars, values)?;
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(table)
}

fn parse_row(input: ParseStream<'_>, vars: &TemplateVars) -> Result<Vec<TokenStream>> {
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
        return Ok(values);
    }

    let value = parse_tokens_until_comma(input)?;

    match vars.len() {
        1 => Ok(vec![value]),
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

struct RangeInput {
    start: u64,
    end: u64,
    inclusive: bool,
    kind: RangeKind,
    suffix: String,
    width: usize,
    radix: RangeRadix,
    tokens: TokenStream,
}

impl RangeInput {
    fn values(&self) -> Vec<TokenStream> {
        if self.start > self.end || (!self.inclusive && self.start == self.end) {
            vec![]
        } else if self.inclusive {
            (self.start..=self.end)
                .filter_map(|v| self.value_to_tokens(v))
                .collect()
        } else {
            (self.start..self.end)
                .filter_map(|v| self.value_to_tokens(v))
                .collect()
        }
    }

    fn value_to_tokens(&self, value: u64) -> Option<TokenStream> {
        match self.kind {
            RangeKind::Integer => {
                let width = self.width;
                let repr = match self.radix {
                    RangeRadix::Binary => format!("0b{:0width$b}{}", value, self.suffix),
                    RangeRadix::Octal => format!("0o{:0width$o}{}", value, self.suffix),
                    RangeRadix::Decimal => format!("{:0width$}{}", value, self.suffix),
                    RangeRadix::LowerHex => format!("0x{:0width$x}{}", value, self.suffix),
                    RangeRadix::UpperHex => format!("0x{:0width$X}{}", value, self.suffix),
                };
                Some(repr.parse().expect("generated range literal should parse"))
            }
            RangeKind::Byte => u8::try_from(value)
                .ok()
                .map(|value| Literal::byte_character(value).into_token_stream()),
            RangeKind::Character => u32::try_from(value)
                .ok()
                .and_then(char::from_u32)
                .map(|value| Literal::character(value).into_token_stream()),
        }
    }
}

impl Parse for RangeInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let start = input.parse::<RangeBound>()?;
        let (inclusive, operator) = if input.peek(Token![..=]) {
            (true, input.parse::<Token![..=]>()?.into_token_stream())
        } else {
            (false, input.parse::<Token![..]>()?.into_token_stream())
        };
        let end = input.parse::<RangeBound>()?;

        let tokens = TokenStream::from_iter([start.tokens.clone(), operator, end.tokens.clone()]);
        if start.kind != end.kind {
            return Err(Error::new_spanned(
                TokenStream::from_iter([start.tokens.clone(), end.tokens.clone()]),
                "range bounds must both be integer literals, both byte literals, or both character literals",
            ));
        }

        let suffix = if start.suffix.is_empty() {
            end.suffix.clone()
        } else if end.suffix.is_empty() || start.suffix == end.suffix {
            start.suffix.clone()
        } else {
            return Err(Error::new_spanned(
                end.tokens.clone(),
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
                end.tokens.clone(),
                "range bounds must use the same integer radix",
            ));
        };

        Ok(Self {
            start: start.value,
            end: end.value,
            inclusive,
            kind: start.kind,
            suffix,
            width: start.width.min(end.width),
            radix,
            tokens,
        })
    }
}

struct RangeBound {
    value: u64,
    kind: RangeKind,
    suffix: String,
    width: usize,
    radix: RangeRadix,
    tokens: TokenStream,
}

impl Parse for RangeBound {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let literal = input.parse::<Lit>()?;
        let tokens = literal.clone().into_token_stream();

        match literal {
            Lit::Int(value) => parse_integer_bound(value),
            Lit::Byte(value) => Ok(Self {
                value: u64::from(value.value()),
                kind: RangeKind::Byte,
                suffix: String::new(),
                width: 0,
                radix: RangeRadix::Decimal,
                tokens,
            }),
            Lit::Char(value) => Ok(Self {
                value: u64::from(u32::from(value.value())),
                kind: RangeKind::Character,
                suffix: String::new(),
                width: 0,
                radix: RangeRadix::Decimal,
                tokens,
            }),
            _ => Err(Error::new_spanned(
                tokens,
                "range bounds must be integer, byte, or character literals",
            )),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeKind {
    Integer,
    Byte,
    Character,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeRadix {
    Binary,
    Octal,
    Decimal,
    LowerHex,
    UpperHex,
}

fn parse_integer_bound(value: syn::LitInt) -> Result<RangeBound> {
    let tokens = value.clone().into_token_stream();
    let repr = value.to_string();

    let (mut radix, base, digits_start) = if repr.starts_with("0b") {
        (RangeRadix::Binary, 2, 2)
    } else if repr.starts_with("0o") {
        (RangeRadix::Octal, 8, 2)
    } else if repr.starts_with("0x") {
        (RangeRadix::LowerHex, 16, 2)
    } else if repr.starts_with("0X") {
        (RangeRadix::UpperHex, 16, 2)
    } else {
        (RangeRadix::Decimal, 10, 0)
    };

    let body = &repr[digits_start..];
    let mut digits = String::new();
    let mut suffix = String::new();

    for (offset, ch) in body.char_indices() {
        match ch {
            '_' => {}
            '0'..='9' => digits.push(ch),
            'A'..='F' if radix == RangeRadix::LowerHex => {
                digits.push(ch);
                radix = RangeRadix::UpperHex;
            }
            'a'..='f' | 'A'..='F' if base == 16 => digits.push(ch),
            _ => {
                if digits.is_empty() {
                    return Err(Error::new_spanned(tokens, "expected integer range bound"));
                }
                suffix = repr[digits_start + offset..].to_owned();
                break;
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
        kind: RangeKind::Integer,
        suffix,
        width: digits.len(),
        radix,
        tokens,
    })
}
