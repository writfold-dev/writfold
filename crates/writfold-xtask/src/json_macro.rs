use verus_syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
};

pub struct Expressions(pub Vec<Expr>);
impl Parse for Expressions {
    fn parse(input: ParseStream<'_>) -> verus_syn::Result<Self> {
        let mut expressions = Vec::new();
        value(input, &mut expressions)?;
        Ok(Self(expressions))
    }
}

fn value(input: ParseStream<'_>, expressions: &mut Vec<Expr>) -> verus_syn::Result<()> {
    if input.peek(verus_syn::token::Brace) {
        let inner;
        verus_syn::braced!(inner in input);
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
    } else if input.peek(verus_syn::token::Bracket) {
        let inner;
        verus_syn::bracketed!(inner in input);
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
