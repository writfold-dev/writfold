use proc_macro2::{TokenStream, TokenTree};
use std::collections::BTreeSet;

pub fn lines(tokens: TokenStream) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    collect(tokens, &mut result);
    result
}

fn collect(tokens: TokenStream, result: &mut BTreeSet<usize>) {
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        if matches!(&token, TokenTree::Punct(p) if p.as_char() == '#') {
            let inner = if matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '!')
            {
                tokens.next()
            } else {
                None
            };
            if let Some(TokenTree::Group(group)) = tokens.peek()
                && matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc")
            {
                tokens.next();
                continue;
            }
            if let Some(inner) = inner {
                result.extend(inner.span().start().line..=inner.span().end().line);
            }
        }
        match token {
            TokenTree::Group(group) => {
                result.insert(group.span_open().start().line);
                result.insert(group.span_close().start().line);
                collect(group.stream(), result);
            }
            other => {
                result.extend(other.span().start().line..=other.span().end().line);
            }
        }
    }
}

pub fn contains_ident(tokens: TokenStream, wanted: &[&str]) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => wanted.iter().any(|word| ident == *word),
        TokenTree::Group(group) => contains_ident(group.stream(), wanted),
        _ => false,
    })
}

/// Match the verifier namespace path, not a field merely named `verifier`.
pub fn contains_verifier_path(tokens: TokenStream) -> bool {
    let mut phase = 0;
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                if contains_verifier_path(group.stream()) {
                    return true;
                }
                phase = 0;
            }
            TokenTree::Ident(ident) => {
                phase = if ident == "verifier" || ident == "r#verifier" {
                    1
                } else {
                    0
                };
            }
            TokenTree::Punct(punct) if punct.as_char() == ':' && phase > 0 => {
                if phase == 2 {
                    return true;
                }
                phase = 2;
            }
            _ => phase = 0,
        }
    }
    false
}
