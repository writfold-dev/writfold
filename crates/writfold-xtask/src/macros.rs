use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use std::collections::BTreeMap;

// Inspect each handwritten expansion once, not a guessed compiler expansion.
// Unsupported substitution grammar fails closed instead of hiding declarations.
pub fn templates(tokens: TokenStream) -> Result<Vec<TokenStream>, String> {
    let tokens: Vec<_> = tokens.into_iter().collect();
    let mut bodies = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if matches!(&tokens[index], TokenTree::Punct(p) if p.as_char() == ';' || p.as_char() == ',')
        {
            index += 1;
            continue;
        }
        let Some(TokenTree::Group(matcher)) = tokens.get(index) else {
            return Err("macro rule has no matcher".into());
        };
        let Some(TokenTree::Group(body)) = tokens.get(index + 3) else {
            return Err("macro rule has no expansion body".into());
        };
        let mut bindings = BTreeMap::new();
        collect_bindings(matcher.stream(), &mut bindings);
        bodies.push(substitute(body.stream(), &bindings)?);
        index += 4;
    }
    if bodies.is_empty() {
        return Err("empty macro definition".into());
    }
    Ok(bodies)
}

fn collect_bindings(tokens: TokenStream, bindings: &mut BTreeMap<String, String>) {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for window in tokens.windows(4) {
        if let [
            TokenTree::Punct(dollar),
            TokenTree::Ident(name),
            TokenTree::Punct(colon),
            TokenTree::Ident(kind),
        ] = window
            && dollar.as_char() == '$'
            && colon.as_char() == ':'
        {
            bindings.insert(name.to_string(), kind.to_string());
        }
    }
    for token in tokens {
        if let TokenTree::Group(group) = token {
            collect_bindings(group.stream(), bindings);
        }
    }
}

fn substitute(
    tokens: TokenStream,
    bindings: &BTreeMap<String, String>,
) -> Result<TokenStream, String> {
    let mut input = tokens.into_iter().peekable();
    let mut output = TokenStream::new();
    while let Some(token) = input.next() {
        if matches!(&token, TokenTree::Punct(p) if p.as_char() == '$') {
            match input.next() {
                Some(TokenTree::Group(group)) => {
                    output.extend(substitute(group.stream(), bindings)?);
                    let mut closed = false;
                    for next in input.by_ref() {
                        if matches!(&next, TokenTree::Punct(p) if matches!(p.as_char(), '*' | '+' | '?'))
                        {
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        return Err("unterminated macro repetition".into());
                    }
                }
                Some(TokenTree::Ident(name)) => {
                    output.extend(replacement(&name, bindings)?);
                }
                _ => return Err("unsupported dollar token".into()),
            }
        } else if let TokenTree::Group(group) = token {
            let mut rebuilt = Group::new(group.delimiter(), substitute(group.stream(), bindings)?);
            rebuilt.set_span(group.span());
            output.extend([TokenTree::Group(rebuilt)]);
        } else {
            output.extend([token]);
        }
    }
    Ok(output)
}

fn replacement(name: &Ident, bindings: &BTreeMap<String, String>) -> Result<TokenStream, String> {
    let key = name.to_string();
    let kind = bindings.get(&key).map(String::as_str);
    let replacement = match kind {
        Some("ident") => format!("__wf_{key}"),
        Some("ty" | "path") => "u64".into(),
        Some("expr" | "expr_2021" | "literal") => "0".into(),
        Some("meta") => "doc = \"macro metadata\"".into(),
        Some("lifetime") => "'wf".into(),
        Some("vis") => "pub".into(),
        _ => return Err(format!("unsupported macro fragment {key}: {kind:?}")),
    };
    let parsed: TokenStream = replacement.parse().map_err(|error| format!("{error}"))?;
    Ok(parsed
        .into_iter()
        .map(|mut token| {
            token.set_span(name.span());
            token
        })
        .collect())
}

pub fn declared_name(item: &verus_syn::ItemMacro) -> Option<&Ident> {
    if item.mac.path.is_ident("macro_rules") {
        item.ident.as_ref()
    } else {
        None
    }
}
