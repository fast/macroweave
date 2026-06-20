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
use proc_macro2::Group;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use syn::Result;

use crate::parse::Binding;
use crate::parse::Table;
use crate::parse::Template;
use crate::parse::substitute;

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let template = syn::parse2::<Template>(input)?;
    let (table, template) = template.into_parts();

    let mut found_splice = false;
    let expanded = expand_splices(&table, template.clone(), &mut found_splice);
    if found_splice {
        return Ok(expanded);
    }

    let mut output = TokenStream::new();
    for bindings in table.rows() {
        output.extend(substitute_tokens(bindings, template.clone()));
    }
    Ok(output)
}

fn substitute_tokens(bindings: &[Binding], tokens: TokenStream) -> TokenStream {
    let mut new_tokens = TokenStream::new();
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let content = substitute_tokens(bindings, group.stream());
                let mut new_group = Group::new(group.delimiter(), content);
                new_group.set_span(group.span());
                new_tokens.extend([TokenTree::Group(new_group)]);
            }
            TokenTree::Ident(ident) => new_tokens.extend(substitute(ident, bindings)),
            other => new_tokens.extend([other]),
        }
    }
    new_tokens
}

fn expand_splices(table: &Table, tokens: TokenStream, found_splice: &mut bool) -> TokenStream {
    let mut tokens = tokens.into_iter().collect::<Vec<_>>();

    let mut i = 0;
    while i < tokens.len() {
        if let TokenTree::Group(group) = &mut tokens[i] {
            let content = expand_splices(table, group.stream(), found_splice);
            let mut new_group = Group::new(group.delimiter(), content);
            new_group.set_span(group.span());
            *group = new_group;
            i += 1;
            continue;
        }

        let Some(splice) = enter_hash_repetition(&tokens[i..]) else {
            i += 1;
            continue;
        };

        *found_splice = true;
        let mut repeated = vec![];
        for (row, bindings) in table.rows().enumerate() {
            if row > 0 {
                if let Some(separator) = &splice.separator {
                    repeated.push(separator.clone());
                }
            }
            repeated.extend(substitute_tokens(bindings, splice.template.clone()));
        }

        let repeated_len = repeated.len();
        tokens.splice(i..i + splice.consumed_len, repeated);
        i += repeated_len;
    }

    tokens.into_iter().collect()
}

struct Splice {
    template: TokenStream,
    separator: Option<TokenTree>,
    consumed_len: usize,
}

fn enter_hash_repetition(tokens: &[TokenTree]) -> Option<Splice> {
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
