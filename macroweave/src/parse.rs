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

use proc_macro2::Delimiter;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use syn::Error;
use syn::Result;
use syn::Token;
use syn::braced;
use syn::bracketed;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

pub struct Invocation {
    pub table: Table,
    pub body: TokenStream,
}

impl Parse for Invocation {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let pattern = input.parse::<Pattern>()?;
        input.parse::<Token![in]>()?;

        let rows;
        let bracket = bracketed!(rows in input);
        let table = parse_rows(&rows, &pattern, bracket.span.join())?;

        let body;
        braced!(body in input);
        let body = body.parse::<TokenStream>()?;

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after macro body"));
        }

        Ok(Self { table, body })
    }
}

pub struct Table {
    pub names: Vec<Ident>,
    pub rows: Vec<Vec<TokenStream>>,
}

enum Pattern {
    Single(Slot),
    Tuple(Vec<Slot>),
}

impl Pattern {
    fn slots(&self) -> &[Slot] {
        match self {
            Self::Single(slot) => std::slice::from_ref(slot),
            Self::Tuple(slots) => slots,
        }
    }

    fn names(&self) -> Vec<Ident> {
        let mut names = vec![];
        for slot in self.slots() {
            if let Slot::Bind(ident) = slot {
                names.push(ident.clone());
            }
        }
        names
    }

    fn bind(&self, values: Vec<TokenStream>) -> Result<Vec<TokenStream>> {
        let slots = self.slots();
        if values.len() != slots.len() {
            let expected = slots.len();
            let found = values.len();
            return Err(Error::new_spanned(
                values.into_iter().collect::<TokenStream>(),
                format!(
                    "this row provides {found} value{}, but the binding pattern expects {expected}",
                    if found == 1 { "" } else { "s" }
                ),
            ));
        }

        let mut bound_values = vec![];
        for (slot, value) in slots.iter().zip(values) {
            if matches!(slot, Slot::Bind(_)) {
                bound_values.push(value);
            }
        }
        Ok(bound_values)
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let pattern = if input.peek(syn::token::Paren) {
            let content;
            let paren = parenthesized!(content in input);
            let slots = parse_slots(&content)?;
            if slots.is_empty() {
                return Err(Error::new(
                    paren.span.join(),
                    "expected at least one binding in tuple pattern",
                ));
            }
            Self::Tuple(slots)
        } else {
            Self::Single(input.parse::<Slot>()?)
        };

        check_duplicate_names(pattern.slots())?;
        Ok(pattern)
    }
}

enum Slot {
    Bind(Ident),
    Ignore,
}

impl Parse for Slot {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![_]) {
            input.parse::<Token![_]>()?;
            return Ok(Self::Ignore);
        }

        Ok(Self::Bind(input.parse::<Ident>()?))
    }
}

fn parse_slots(input: ParseStream<'_>) -> Result<Vec<Slot>> {
    Punctuated::<Slot, Token![,]>::parse_terminated(input)
        .map(Punctuated::into_iter)
        .map(Iterator::collect)
}

fn check_duplicate_names(slots: &[Slot]) -> Result<()> {
    let mut names: Vec<&Ident> = vec![];
    for slot in slots {
        let Slot::Bind(ident) = slot else {
            continue;
        };

        if let Some(previous) = names.iter().copied().find(|previous| previous == &ident) {
            let mut error = Error::new_spanned(
                ident,
                format!("the binding variable `{ident}` is declared more than once"),
            );
            error.combine(Error::new_spanned(
                previous,
                format!("the first `{previous}` binding is here"),
            ));
            return Err(error);
        }

        names.push(ident);
    }

    Ok(())
}

fn parse_rows(input: ParseStream<'_>, pattern: &Pattern, span: proc_macro2::Span) -> Result<Table> {
    let tokens = input.parse::<TokenStream>()?;
    let rows = match pattern {
        Pattern::Single(_) => split_values(tokens)?
            .into_iter()
            .map(|value| pattern.bind(vec![value]))
            .collect::<Result<Vec<_>>>()?,
        Pattern::Tuple(_) => split_values(tokens)?
            .into_iter()
            .map(|row| parse_tuple_row(row).and_then(|values| pattern.bind(values)))
            .collect::<Result<Vec<_>>>()?,
    };

    if rows.is_empty() {
        return Err(Error::new(span, "input list must contain at least one row"));
    }

    Ok(Table {
        rows,
        names: pattern.names(),
    })
}

fn parse_tuple_row(tokens: TokenStream) -> Result<Vec<TokenStream>> {
    let mut iter = tokens.clone().into_iter();
    let Some(TokenTree::Group(group)) = iter.next() else {
        return Err(Error::new_spanned(
            tokens,
            "rows for tuple bindings must use parentheses, such as `(u16, Small)`",
        ));
    };

    if group.delimiter() != Delimiter::Parenthesis || iter.next().is_some() {
        return Err(Error::new_spanned(
            tokens,
            "rows for tuple bindings must use parentheses, such as `(u16, Small)`",
        ));
    }

    split_values(group.stream())
}

fn split_values(tokens: TokenStream) -> Result<Vec<TokenStream>> {
    let mut values = vec![];
    let mut value = TokenStream::new();
    let mut angle_depth = 0usize;

    for token in tokens {
        if is_comma(&token) && angle_depth == 0 {
            if is_empty(&value) {
                return Err(Error::new_spanned(token, "expected value before comma"));
            }

            values.push(value);
            value = TokenStream::new();
            continue;
        }

        update_angle_depth(&token, &mut angle_depth);
        value.extend([token]);
    }

    if !is_empty(&value) {
        values.push(value);
    }

    Ok(values)
}

fn update_angle_depth(token: &TokenTree, angle_depth: &mut usize) {
    let TokenTree::Punct(punct) = token else {
        return;
    };

    match punct.as_char() {
        '<' => *angle_depth += 1,
        '>' => *angle_depth = angle_depth.saturating_sub(1),
        _ => {}
    }
}

fn is_comma(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',')
}

fn is_empty(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().next().is_none()
}
