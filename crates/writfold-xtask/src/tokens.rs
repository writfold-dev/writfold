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
