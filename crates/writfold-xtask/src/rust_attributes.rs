use crate::{syntax::Audit, tokens};
use syn::{Attribute, Meta, MetaList, Path, parse::Parser, spanned::Spanned};

pub fn check(audit: &mut Audit<'_>, attribute: &Attribute) {
    let root = attribute
        .path()
        .segments
        .first()
        .map(|part| part.ident.to_string());
    if !root.as_deref().is_some_and(known) {
        audit.emit(
            "opaque-macro",
            attribute.span(),
            "unclassified attribute-generated declarations",
        );
    }
    if root.as_deref() == Some("verifier") {
        let name = attribute
            .path()
            .segments
            .last()
            .map(|part| part.ident.to_string());
        audit.proof_attribute(name.as_deref(), attribute.span());
    }
    if attribute.path().is_ident("unsafe") {
        audit.emit("unsafe", attribute.span(), "unsafe attribute");
    }
    if attribute.path().is_ident("path") {
        let value = if let Meta::NameValue(entry) = &attribute.meta
            && let syn::Expr::Lit(expression) = &entry.value
            && let syn::Lit::Str(value) = &expression.lit
        {
            Some(value.value())
        } else {
            None
        };
        audit.module_path(attribute.span(), value.as_deref());
    }
    let Meta::List(list) = &attribute.meta else {
        return;
    };
    if list.path.is_ident("cfg_attr") {
        conditional(audit, list);
    }
    if list.path.is_ident("derive") {
        derives(audit, list);
    }
    if matches!(root.as_deref(), Some("allow" | "expect")) {
        let broad = tokens::contains_ident(
            list.tokens.clone(),
            &["unsafe_code", "unsafe_op_in_unsafe_fn", "warnings", "all"],
        );
        let panic = !audit.test
            && tokens::contains_ident(
                list.tokens.clone(),
                &[
                    "unwrap_used",
                    "expect_used",
                    "panic",
                    "todo",
                    "unimplemented",
                ],
            );
        if broad || panic {
            audit.emit(
                "unsafe-allowance",
                attribute.span(),
                "broad or prohibited lint allowance",
            );
        }
    }
}

fn known(name: &str) -> bool {
    matches!(
        name,
        "doc"
            | "derive"
            | "cfg"
            | "cfg_attr"
            | "allow"
            | "deny"
            | "forbid"
            | "expect"
            | "repr"
            | "inline"
            // crate-level `#![no_std]` (writfold-core) generates no declarations
            | "no_std"
            | "cold"
            | "must_use"
            | "deprecated"
            | "non_exhaustive"
            | "track_caller"
            | "test"
            | "should_panic"
            | "ignore"
            | "path"
            | "unsafe"
            | "error"
            | "from"
            | "source"
            | "backtrace"
            | "serde"
            | "verifier"
            | "rustfmt"
    )
}

fn conditional(audit: &mut Audit<'_>, list: &MetaList) {
    let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
    match parser.parse2(list.tokens.clone()) {
        Ok(metas) => {
            for meta in metas.into_iter().skip(1) {
                let attribute = Attribute {
                    pound_token: Default::default(),
                    style: syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    meta,
                };
                check(audit, &attribute);
            }
        }
        Err(error) => audit.emit("macro-parse", list.span(), error.to_string()),
    }
}

fn derives(audit: &mut Audit<'_>, list: &MetaList) {
    let parser = syn::punctuated::Punctuated::<Path, syn::Token![,]>::parse_terminated;
    match parser.parse2(list.tokens.clone()) {
        Ok(paths) => {
            for path in paths {
                let name = path.segments.last().map(|part| part.ident.to_string());
                if audit.package == "writfold-domain" && name.as_deref() == Some("Deserialize") {
                    audit.emit(
                        "domain-deserialize",
                        path.span(),
                        "unchecked domain Deserialize derive",
                    );
                }
                if !matches!(
                    name.as_deref(),
                    Some(
                        "Clone"
                            | "Copy"
                            | "Debug"
                            | "Default"
                            | "Eq"
                            | "PartialEq"
                            | "Ord"
                            | "PartialOrd"
                            | "Hash"
                            | "Error"
                            | "Serialize"
                            | "Deserialize"
                    )
                ) {
                    audit.emit(
                        "opaque-macro",
                        path.span(),
                        "unclassified derive-generated declarations",
                    );
                }
            }
        }
        Err(error) => audit.emit("macro-parse", list.span(), error.to_string()),
    }
}
