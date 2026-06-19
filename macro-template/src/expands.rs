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
use syn::Result;
use syn::braced;
use syn::parse::Parse;
use syn::parse::ParseStream;

use crate::sources::Sources;

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let template = syn::parse2::<Template>(input)?;
    Ok(template.expand())
}

#[derive(Clone)]
pub struct Replacement {
    placeholder: Ident,
    tokens: TokenStream,
}

impl Replacement {
    pub(crate) fn new(placeholder: Ident, tokens: TokenStream) -> Self {
        Self {
            placeholder,
            tokens,
        }
    }

    pub(crate) fn placeholder(&self) -> &Ident {
        &self.placeholder
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
    fn expand(self) -> TokenStream {
        let Self {
            sources: Sources { rows },
            template,
        } = self;

        let replacements = rows
            .iter()
            .map(|src| src.replacements.as_slice())
            .collect::<Vec<_>>();

        let mut found_splice = false;
        let expanded = expand_splice_blocks(&replacements, template.clone(), &mut found_splice);
        if found_splice {
            expanded
        } else {
            let mut output = TokenStream::new();
            for replacement in replacements {
                output.extend(replace_token_stream(replacement, template.clone()));
            }
            output
        }
    }
}

fn replace_token_stream(replacements: &[Replacement], tokens: TokenStream) -> TokenStream {
    let mut new_tokens = TokenStream::new();
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let content = replace_token_stream(replacements, group.stream());
                let mut new_group = Group::new(group.delimiter(), content);
                new_group.set_span(group.span());
                new_tokens.extend([TokenTree::Group(new_group)]);
            }
            TokenTree::Ident(ident) => {
                if let Ok(index) = replacements
                    .binary_search_by(|replacement| replacement.placeholder().cmp(&ident))
                {
                    new_tokens.extend(replacements[index].tokens.clone());
                } else {
                    new_tokens.extend([TokenTree::Ident(ident)]);
                }
            }
            other => new_tokens.extend([other]),
        }
    }
    new_tokens
}

fn expand_splice_blocks(
    replacements_by_row: &[&[Replacement]],
    tokens: TokenStream,
    found_splice: &mut bool,
) -> TokenStream {
    let mut tokens = tokens.into_iter().collect::<Vec<_>>();

    let mut i = 0;
    while i < tokens.len() {
        if let TokenTree::Group(group) = &mut tokens[i] {
            let content = expand_splice_blocks(replacements_by_row, group.stream(), found_splice);
            let mut new_group = Group::new(group.delimiter(), content);
            new_group.set_span(group.span());
            *group = new_group;
            i += 1;
            continue;
        }

        let Some(template) = enter_splice_block(&tokens[i..]) else {
            i += 1;
            continue;
        };

        *found_splice = true;
        let mut repeated = vec![];
        for row_replacements in replacements_by_row {
            repeated.extend(replace_token_stream(row_replacements, template.clone()));
        }

        let repeated_len = repeated.len();
        tokens.splice(i..i + 3, repeated);
        i += repeated_len;
    }

    tokens.into_iter().collect()
}

fn enter_splice_block(tokens: &[TokenTree]) -> Option<TokenStream> {
    let [
        TokenTree::Punct(at),
        TokenTree::Ident(ident),
        TokenTree::Group(group),
        ..,
    ] = tokens
    else {
        return None;
    };
    if at.as_char() != '@' || ident != "splice" || group.delimiter() != Delimiter::Brace {
        return None;
    }
    Some(group.stream())
}
