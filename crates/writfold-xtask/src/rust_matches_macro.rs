use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream},
};

pub struct Arguments {
    pub value: Expr,
    pub pattern: Pat,
    pub guard: Option<Expr>,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let value = input.parse()?;
        input.parse::<Token![,]>()?;
        let pattern = Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            value,
            pattern,
            guard,
        })
    }
}
