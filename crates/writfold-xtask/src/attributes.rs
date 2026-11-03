use crate::{syntax::Audit, tokens};
use verus_syn::{Attribute, Meta, MetaList, Path, parse::Parser, spanned::Spanned};

pub fn check(audit: &mut Audit<'_>, attribute: &Attribute) {
    check_owned(audit, attribute, false);
}

pub fn check_function(
    audit: &mut Audit<'_>,
    attribute: &Attribute,
    signature: &verus_syn::Signature,
    has_body: bool,
) {
    let spec_body = has_body
        && matches!(
            signature.mode,
            verus_syn::FnMode::Spec(_) | verus_syn::FnMode::SpecChecked(_)
        );
    check_owned(audit, attribute, spec_body);
}

fn check_owned(audit: &mut Audit<'_>, attribute: &Attribute, spec_body: bool) {
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
        // Opacity controls unfolding only on a spec definition. Ownership is
        // passed through cfg_attr, never inherited by attributes in the body
        // or by unexpanded macro arguments.
        let unfolding = spec_body
            && name.as_deref() == Some("opaque")
            && attribute.path().segments.len() == 2
            && matches!(&attribute.meta, Meta::Path(_));
        if !unfolding {
            audit.proof_attribute(name.as_deref(), attribute.span());
        }
    }
    if attribute.path().is_ident("unsafe") {
        audit.emit("unsafe", attribute.span(), "unsafe attribute");
    }
    if attribute.path().is_ident("path") {
        let value = if let Meta::NameValue(entry) = &attribute.meta
            && let verus_syn::Expr::Lit(expression) = &entry.value
            && let verus_syn::Lit::Str(value) = &expression.lit
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
        conditional(audit, list, spec_body);
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
            | "trigger"
    )
}

fn conditional(audit: &mut Audit<'_>, list: &MetaList, spec_body: bool) {
    let parser = verus_syn::punctuated::Punctuated::<Meta, verus_syn::Token![,]>::parse_terminated;
    match parser.parse2(list.tokens.clone()) {
        Ok(metas) => {
            for meta in metas.into_iter().skip(1) {
                let attribute = Attribute {
                    pound_token: Default::default(),
                    style: verus_syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    meta,
                };
                check_owned(audit, &attribute, spec_body);
            }
        }
        Err(error) => audit.emit("macro-parse", list.span(), error.to_string()),
    }
}

fn derives(audit: &mut Audit<'_>, list: &MetaList) {
    let parser = verus_syn::punctuated::Punctuated::<Path, verus_syn::Token![,]>::parse_terminated;
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
