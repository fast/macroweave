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
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use syn::Error;
use syn::Result;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;

use crate::sources::Sources;

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    syn::parse2::<Template>(input)?.expand()
}

#[derive(Clone)]
pub struct Replacement {
    placeholder: Ident,
    tokens: TokenStream,
}

impl Replacement {
    pub fn new(placeholder: Ident, tokens: TokenStream) -> Self {
        Self {
            placeholder,
            tokens,
        }
    }
}

struct Template {
    sources: Sources,
    template: TokenStream,
}

impl Parse for Template {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // template! {
        // <!-- source start -->
        //     for (Endian, Method) in [
        //         (LittleEndian, to_le_bytes),
        //         (BigEndian, to_be_bytes),
        //         (NativeEndian, to_ne_bytes)
        //     ],
        //     for (Ty, Width) in [
        //         (u16, 2),
        //         (u32, 4),
        //     ],
        // <!-- source end -->
        let sources = input.parse::<Sources>()?;
        // <!-- template start -->
        //     {
        //         impl StoreBytes<Endian, Width> for Ty {
        //             fn store_bytes(&self) -> [u8; Width] {
        //                 self.Method()
        //             }
        //         }
        //     }
        // <!-- template end -->
        let template;
        braced!(template in input);
        let template = template.parse::<TokenStream>()?;
        // }
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after template body"));
        }
        Ok(Self { sources, template })
    }
}

impl Template {
    fn expand(self) -> Result<TokenStream> {
        let Self {
            sources: Sources { placeholders, rows },
            template,
        } = self;

        let replacements = rows
            .iter()
            .map(|src| src.replacements.as_slice())
            .collect::<Vec<_>>();

        if contains_splice_block(template.clone()) {
            let common_replacements = common_replacements(&placeholders, &replacements);
            return expand_splice_blocks(
                &replacements,
                &common_replacements,
                &placeholders,
                template,
            );
        }

        let mut output = TokenStream::new();
        for replacement in replacements {
            output.extend(replace_token_stream(template.clone(), replacement));
        }
        Ok(output)
    }
}

fn replace_token_stream(tokens: TokenStream, replacements: &[Replacement]) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Ident(ident) => replacements
                .iter()
                .find_map(|replacement| {
                    if ident == replacement.placeholder {
                        Some(replacement.tokens.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| ident.into_token_stream()),
            TokenTree::Group(group) => {
                let mut new_group = Group::new(
                    group.delimiter(),
                    replace_token_stream(group.stream(), replacements),
                );
                new_group.set_span(group.span());
                new_group.into_token_stream()
            }
            other => other.into(),
        })
        .collect()
}

fn common_replacements(
    placeholders: &[Ident],
    replacements_by_row: &[&[Replacement]],
) -> Vec<Replacement> {
    placeholders
        .iter()
        .filter_map(|placeholder| common_replacement(placeholder, replacements_by_row))
        .collect()
}

fn common_replacement(
    placeholder: &Ident,
    replacements_by_row: &[&[Replacement]],
) -> Option<Replacement> {
    let mut rows = replacements_by_row.iter();
    let first = rows
        .next()?
        .iter()
        .find(|replacement| replacement.placeholder == *placeholder)?
        .clone();

    for row in rows {
        let replacement = row
            .iter()
            .find(|replacement| replacement.placeholder == *placeholder)?;

        let first = first.tokens.to_string();
        let replacement = replacement.tokens.to_string();
        if first != replacement {
            return None;
        }
    }

    Some(first)
}

fn contains_splice_block(tokens: TokenStream) -> bool {
    let mut tokens = tokens.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(at) if at.as_char() == '@' => {
                let mut lookahead = tokens.clone();
                if splice_block(&mut lookahead).is_some() {
                    return true;
                }
            }
            TokenTree::Group(group) if contains_splice_block(group.stream()) => {
                return true;
            }
            _ => {}
        }
    }

    false
}

fn expand_splice_blocks(
    replacements_by_row: &[&[Replacement]],
    common_replacements: &[Replacement],
    placeholders: &[Ident],
    tokens: TokenStream,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let mut tokens = tokens.into_iter().peekable();

    while let Some(token) = tokens.next() {
        if let TokenTree::Punct(at) = &token {
            if at.as_char() == '@' {
                let mut lookahead = tokens.clone();
                if let Some(template) = splice_block(&mut lookahead) {
                    tokens = lookahead;
                    for row_replacements in replacements_by_row {
                        output.extend(replace_token_stream(template.clone(), row_replacements));
                    }
                    continue;
                }
            }
        }

        output.extend(replace_common_token(
            token,
            common_replacements,
            placeholders,
            replacements_by_row,
        )?);
    }

    Ok(output)
}

fn replace_common_token(
    token: TokenTree,
    common_replacements: &[Replacement],
    placeholders: &[Ident],
    replacements_by_row: &[&[Replacement]],
) -> Result<TokenStream> {
    match token {
        TokenTree::Ident(ident) => {
            if let Some(replacement) = common_replacements
                .iter()
                .find(|replacement| ident == replacement.placeholder)
            {
                return Ok(replacement.tokens.clone());
            }
            if placeholders.contains(&ident) {
                return Err(Error::new_spanned(
                    ident,
                    "row-varying placeholder must appear inside an `@splice { ... }` block",
                ));
            }
            Ok(ident.into_token_stream())
        }
        TokenTree::Group(group) => {
            let content = expand_splice_blocks(
                replacements_by_row,
                common_replacements,
                placeholders,
                group.stream(),
            )?;

            let mut new_group = Group::new(group.delimiter(), content);
            new_group.set_span(group.span());
            Ok(new_group.into_token_stream())
        }
        other => Ok(other.into_token_stream()),
    }
}

fn splice_block(tokens: &mut impl Iterator<Item = TokenTree>) -> Option<TokenStream> {
    let Some(TokenTree::Ident(ident)) = tokens.next() else {
        return None;
    };
    if ident != "splice" {
        return None;
    }

    let Some(TokenTree::Group(group)) = tokens.next() else {
        return None;
    };
    if group.delimiter() != Delimiter::Brace {
        return None;
    }
    Some(group.stream())
}
