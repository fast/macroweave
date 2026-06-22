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
    pub rows: Vec<Vec<Ident>>,
}

struct Pattern {
    slots: Vec<Slot>,
}

impl Pattern {
    fn arity(&self) -> usize {
        self.slots.len()
    }

    fn names(&self) -> Vec<Ident> {
        let mut names = vec![];
        for slot in &self.slots {
            if let Slot::Bind(ident) = slot {
                names.push(ident.clone());
            }
        }
        names
    }

    fn bind(&self, values: Vec<Ident>) -> Result<Vec<Ident>> {
        if values.len() != self.slots.len() {
            let expected = self.slots.len();
            let found = values.len();
            return Err(Error::new_spanned(
                TokenStream::from_iter(values.into_iter().map(TokenTree::Ident)),
                format!(
                    "this row provides {found} value{}, but the binding pattern expects {expected}",
                    if found == 1 { "" } else { "s" }
                ),
            ));
        }

        let mut bound_values = vec![];
        for (slot, value) in self.slots.iter().zip(values) {
            if matches!(slot, Slot::Bind(_)) {
                bound_values.push(value);
            }
        }
        Ok(bound_values)
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let slots = if input.peek(syn::token::Paren) {
            let content;
            let paren = parenthesized!(content in input);
            let slots = parse_slots(&content)?;
            if slots.is_empty() {
                return Err(Error::new(
                    paren.span.join(),
                    "expected at least one binding in tuple pattern",
                ));
            }
            slots
        } else {
            vec![input.parse::<Slot>()?]
        };

        check_duplicate_names(&slots)?;
        Ok(Self { slots })
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

        if !input.peek(Token![#]) {
            return Err(input.error("binding variables must be written as `#name`"));
        }

        input.parse::<Token![#]>()?;
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
                format!("the binding variable `#{ident}` is declared more than once"),
            );
            error.combine(Error::new_spanned(
                previous,
                format!("the first `#{previous}` binding is here"),
            ));
            return Err(error);
        }

        names.push(ident);
    }

    Ok(())
}

fn parse_rows(input: ParseStream<'_>, pattern: &Pattern, span: proc_macro2::Span) -> Result<Table> {
    let rows = if pattern.arity() == 1 {
        Punctuated::<Ident, Token![,]>::parse_terminated(input)?
            .into_iter()
            .map(|value| pattern.bind(vec![value]))
            .collect::<Result<Vec<_>>>()?
    } else {
        Punctuated::<Row, Token![,]>::parse_terminated(input)?
            .into_iter()
            .map(|row| pattern.bind(row.values))
            .collect::<Result<Vec<_>>>()?
    };

    if rows.is_empty() {
        return Err(Error::new(span, "input list must contain at least one row"));
    }

    Ok(Table {
        rows,
        names: pattern.names(),
    })
}

struct Row {
    values: Vec<Ident>,
}

impl Parse for Row {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Paren) {
            return Err(
                input.error("rows for tuple bindings must use parentheses, such as `(u16, Small)`")
            );
        }

        let row;
        parenthesized!(row in input);
        let values = Punctuated::<Ident, Token![,]>::parse_terminated(&row)?
            .into_iter()
            .collect();
        Ok(Self { values })
    }
}
