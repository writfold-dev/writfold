use crate::syntax::Audit;
use verus_syn::{Path, UseTree, spanned::Spanned};

pub fn path(audit: &mut Audit<'_>, path: &Path) {
    let joined = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    inspect(audit, &joined, path.span());
    if !audit.test
        && !audit.ghost
        && path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "unwrap" | "expect" | "unwrap_unchecked"
            )
        })
    {
        audit.emit(
            "production-panic",
            path.span(),
            "panic-producing associated function",
        );
    }
}

pub fn inspect(audit: &mut Audit<'_>, path: &str, span: proc_macro2::Span) {
    let pure = matches!(
        audit.package,
        "writfold-domain" | "writfold-kernel" | "writfold-protocol"
    );
    if pure && !audit.test {
        let mut segments = path.split("::");
        if segments.next() == Some("std")
            && matches!(
                segments.next(),
                Some("fs" | "net" | "process" | "env" | "time" | "thread" | "io")
            )
        {
            audit.emit("core-io", span, format!("ambient IO dependency {path}"));
        }
    }
    if audit.package == "writfold-domain" && path.split("::").any(|part| part == "Deserialize") {
        audit.emit(
            "domain-deserialize",
            span,
            "domain deserializer import or implementation",
        );
    }
    if matches!(
        audit.package,
        "writfold-domain"
            | "writfold-kernel"
            | "writfold-protocol"
            | "writfold-app"
            | "writfold-store"
    ) && (path == "serde_json::Value" || path.starts_with("serde_json::value::Value"))
    {
        audit.emit(
            "domain-value",
            span,
            "untyped JSON outside the bounded wire/catalog boundary",
        );
    }
}

pub fn imports(audit: &mut Audit<'_>, tree: &UseTree, prefix: String) {
    match tree {
        UseTree::Path(path) => imports(audit, &path.tree, format!("{prefix}{}::", path.ident)),
        UseTree::Name(name) => inspect(audit, &format!("{prefix}{}", name.ident), tree.span()),
        UseTree::Rename(rename) => {
            if prefix.is_empty() && rename.ident == "std" {
                audit.emit(
                    "core-io",
                    tree.span(),
                    "renamed std root conceals effect paths",
                );
            }
            inspect(audit, &format!("{prefix}{}", rename.ident), tree.span());
        }
        UseTree::Glob(_) => {
            if prefix == "std::" {
                audit.emit(
                    "core-io",
                    tree.span(),
                    "glob std import conceals effect paths",
                );
            }
            inspect(audit, prefix.trim_end_matches("::"), tree.span());
        }
        UseTree::Group(group) => {
            for child in &group.items {
                imports(audit, child, prefix.clone());
            }
        }
    }
}
