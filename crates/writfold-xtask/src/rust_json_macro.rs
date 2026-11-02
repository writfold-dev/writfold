use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
};

pub struct Expressions(pub Vec<Expr>);
impl Parse for Expressions {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = Vec::new();
        value(input, &mut expressions)?;
        Ok(Self(expressions))
    }
}

fn value(input: ParseStream<'_>, expressions: &mut Vec<Expr>) -> syn::Result<()> {
    if input.peek(syn::token::Brace) {
        let inner;
        syn::braced!(inner in input);
        while !inner.is_empty() {
            let key: Expr = inner.parse()?;
            expressions.push(key);
            inner.parse::<Token![:]>()?;
            value(&inner, expressions)?;
            if inner.is_empty() {
                break;
            }
            inner.parse::<Token![,]>()?;
        }
    } else if input.peek(syn::token::Bracket) {
        let inner;
        syn::bracketed!(inner in input);
        while !inner.is_empty() {
            value(&inner, expressions)?;
            if inner.is_empty() {
                break;
            }
            inner.parse::<Token![,]>()?;
        }
    } else {
        expressions.push(input.parse()?);
    }
    Ok(())
}
