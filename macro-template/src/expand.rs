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

use std::collections::BTreeSet;

use proc_macro2::Delimiter;
use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use syn::Error;
use syn::Result;

use crate::parse::Invocation;
use crate::parse::Table;

pub fn repeat(input: TokenStream) -> Result<TokenStream> {
    let invocation = syn::parse2::<Invocation>(input)?;

    let mut output = TokenStream::new();
    for row in &invocation.table.rows {
        output.extend(substitute_tokens(
            &invocation.table.names,
            row,
            invocation.body.clone(),
        ));
    }
    Ok(output)
}

pub fn splice(input: TokenStream) -> Result<TokenStream> {
    let invocation = syn::parse2::<Invocation>(input)?;

    let current_names = invocation.table.names.iter().cloned().collect();
    let (output, found_splice) =
        expand_splices(&invocation.table, &current_names, invocation.body)?;

    if !found_splice {
        return Err(Error::new(
            Span::call_site(),
            "expected at least one `#( ... )*` repetition",
        ));
    }

    Ok(output)
}

fn substitute_tokens(names: &[Ident], row: &[Ident], tokens: TokenStream) -> TokenStream {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut output = TokenStream::new();

    let mut i = 0;
    while i < tokens.len() {
        if let Some((ident, consumed)) = read_hash_ident(&tokens[i..])
            && let Some(index) = find_name(names, ident)
        {
            output.extend([TokenTree::Ident(row[index].clone())]);
            i += consumed;
            continue;
        }

        match &tokens[i] {
            TokenTree::Group(group) => {
                let stream = substitute_tokens(names, row, group.stream());
                output.extend([TokenTree::Group(group_with_stream(group, stream))]);
            }
            token => output.extend([token.clone()]),
        }
        i += 1;
    }

    output
}

fn expand_splices(
    table: &Table,
    current_names: &BTreeSet<Ident>,
    tokens: TokenStream,
) -> Result<(TokenStream, bool)> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut output = TokenStream::new();
    let mut found_splice = false;

    let mut i = 0;
    while i < tokens.len() {
        if let Some(splice) = read_hash_repetition(&tokens[i..]) {
            found_splice = true;

            for (row_index, row) in table.rows.iter().enumerate() {
                if row_index > 0
                    && let Some(separator) = &splice.separator
                {
                    output.extend([separator.clone()]);
                }
                output.extend(substitute_tokens(
                    &table.names,
                    row,
                    splice.template.clone(),
                ));
            }

            i += splice.consumed_len;
            continue;
        }

        if let Some((ident, consumed)) = read_hash_ident(&tokens[i..])
            && current_names.contains(ident)
        {
            let stream = TokenStream::from_iter(tokens[i..i + consumed].iter().cloned());
            return Err(Error::new_spanned(
                stream,
                format!("splice placeholder `#{ident}` must be used inside `#( ... )*`"),
            ));
        }

        if let TokenTree::Group(group) = &tokens[i] {
            let (stream, group_found_splice) =
                expand_splices(table, current_names, group.stream())?;
            found_splice |= group_found_splice;
            output.extend([TokenTree::Group(group_with_stream(group, stream))]);
        } else {
            output.extend([tokens[i].clone()]);
        }
        i += 1;
    }

    Ok((output, found_splice))
}

struct Splice {
    template: TokenStream,
    separator: Option<TokenTree>,
    consumed_len: usize,
}

fn read_hash_repetition(tokens: &[TokenTree]) -> Option<Splice> {
    let [TokenTree::Punct(hash), TokenTree::Group(group), rest @ ..] = tokens else {
        return None;
    };

    if hash.as_char() != '#' || group.delimiter() != Delimiter::Parenthesis {
        return None;
    }

    match rest {
        [TokenTree::Punct(star), ..] if star.as_char() == '*' => Some(Splice {
            template: group.stream(),
            separator: None,
            consumed_len: 3,
        }),
        [separator, TokenTree::Punct(star), ..] if star.as_char() == '*' => Some(Splice {
            template: group.stream(),
            separator: Some(separator.clone()),
            consumed_len: 4,
        }),
        _ => None,
    }
}

fn read_hash_ident(tokens: &[TokenTree]) -> Option<(&Ident, usize)> {
    let [TokenTree::Punct(hash), TokenTree::Ident(ident), ..] = tokens else {
        return None;
    };

    if hash.as_char() == '#' {
        Some((ident, 2))
    } else {
        None
    }
}

fn find_name(names: &[Ident], ident: &Ident) -> Option<usize> {
    names.iter().position(|name| name == ident)
}

fn group_with_stream(group: &Group, stream: TokenStream) -> Group {
    let mut new_group = Group::new(group.delimiter(), stream);
    new_group.set_span(group.span());
    new_group
}
