//! The convention-literal check (#190): the literals that name a Git or
//! harness convention (`origin`, `refs/remotes/origin`, `"Bash"`,
//! `.claude`) may appear in product code only in a module the claims
//! register in `docs/guide/assurance.md` names: a resolver, which chooses
//! the referent on purpose, or a module where the word is not a convention
//! (a record field, a vocabulary word), with the reason. Test code is not
//! product code and is not checked.
use crate::rust_syntax::is_test;
use serde_json::{Value, json};
use syn::visit::{self, Visit};

/// Where a convention literal is allowed: the module, its class and why.
/// `docs/guide/assurance.md`'s register lists the same modules (a test
/// checks that the two agree).
pub const ALLOWED: [(&str, &str, &str); 11] = [
    (
        "crates/writfold-kb/src/guard/binding.rs",
        "resolver",
        "Claude Code's built-in binding names its own tools",
    ),
    (
        "crates/writfold-kb/src/guard/event.rs",
        "resolver",
        "a map with no kind is Claude Code's own event, whose shell is Bash",
    ),
    (
        "crates/writfold-app/src/kb/writes.rs",
        "not a convention",
        "`origin` is a record field (how a record came to be)",
    ),
    (
        "crates/writfold-app/src/kb/kinds.rs",
        "not a convention",
        "`origin` is a record field",
    ),
    (
        "crates/writfold-kb/src/write/record.rs",
        "not a convention",
        "`origin` is a record field",
    ),
    (
        "crates/writfold-kb/src/proposal/codec.rs",
        "not a convention",
        "`origin` is a proposal field",
    ),
    (
        "crates/writfold-kb/src/gate/receipt.rs",
        "not a convention",
        "`origin` is where a gate definition came from (KB or repository)",
    ),
    (
        "crates/writfold-kb/src/emit_json.rs",
        "not a convention",
        "`origin` is a graph edge field",
    ),
    (
        "crates/writfold-kb/src/verbs/repos.rs",
        "not a convention",
        "`origin` labels a clone's other URL in kb repos",
    ),
    (
        "crates/writfold-core/src/referent.rs",
        "not a convention",
        "`origin` is a vocabulary word the referent kernel recognises in text",
    ),
    (
        "crates/writfold-tui/src/kb_tree.rs",
        "not a convention",
        "`origin` is a graph edge field",
    ),
];

/// Whether a string literal names a convention.
pub fn convention(text: &str) -> bool {
    text == "origin"
        || text == "Bash"
        || text.contains("refs/remotes/origin")
        || text.contains(".claude")
}

struct Literals<'a> {
    path: &'a str,
    diagnostics: &'a mut Vec<Value>,
}

impl<'ast> Visit<'ast> for Literals<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let attrs = match node {
            syn::Item::Mod(m) => &m.attrs,
            syn::Item::Fn(f) => &f.attrs,
            syn::Item::Impl(i) => &i.attrs,
            syn::Item::Const(c) => &c.attrs,
            syn::Item::Static(s) => &s.attrs,
            _ => return visit::visit_item(self, node),
        };
        if !is_test(attrs) {
            visit::visit_item(self, node);
        }
    }

    // Doc comments are prose, not literals the code uses.
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        let text = node.value();
        if convention(&text) {
            let span = node.span();
            self.diagnostics.push(json!({"rule": "convention-literal", "path": self.path,
                "line": span.start().line, "column": span.start().column + 1,
                "message": format!("the convention literal {text:?} outside a module the claims register names (docs/guide/assurance.md, #190): resolve the referent, or add the module to conventions::ALLOWED and the register with the reason"),
                "severity": "error"}));
        }
    }
}

/// Check one product source file.
pub fn check(path: &str, file: &syn::File, test: bool, diagnostics: &mut Vec<Value>) {
    let tooling = path.starts_with("crates/writfold-xtask/");
    if test || tooling || ALLOWED.iter().any(|(module, _, _)| *module == path) {
        return;
    }
    Literals { path, diagnostics }.visit_file(file);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(source: &str) -> usize {
        let Ok(file) = syn::parse_file(source) else {
            return usize::MAX;
        };
        let mut out = Vec::new();
        check("crates/writfold-kb/src/x.rs", &file, false, &mut out);
        out.len()
    }

    #[test]
    fn a_convention_literal_outside_the_register_is_refused() {
        assert_eq!(
            found("fn f() -> &'static str { \"refs/remotes/origin/main\" }"),
            1
        );
        assert_eq!(found("fn f() -> bool { \"x\" == \"Bash\" }"), 1);
        assert_eq!(found("fn f() -> &'static str { \"original\" }"), 0);
        assert_eq!(
            found("#[cfg(test)] mod t { fn f() -> &'static str { \"origin\" } }"),
            0
        );
    }

    /// The register in `docs/guide/assurance.md` lists every word below
    /// with a theorem or "tested only", names only theorems that are
    /// claimed, and lists exactly the modules of [`ALLOWED`].
    #[test]
    fn the_claims_register_agrees_with_the_code() -> Result<(), String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |p: &str| std::fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
        let page = read("docs/guide/assurance.md")?;
        let claims = read("proofs/claims.toml")?;
        let start = page.find("## Claims register").ok_or("no register")?;
        let end = page[start..]
            .find("\n## ")
            .map_or(page.len(), |e| start + e);
        let register = &page[start..end];
        let words = [
            "`complete`",
            "`fresh`",
            "`gone`",
            "`changed`",
            "`moved`",
            "`ready`",
            "`met`",
            "`pass`",
            "`covered`",
            "no rule matched",
            "`packet=complete`",
            "`coverage=complete`",
        ];
        for word in words {
            let row = register
                .lines()
                .find(|l| l.starts_with(&format!("| {word} |")))
                .ok_or(format!("no row for {word}"))?;
            let theorems: Vec<&str> = row
                .rsplit('|')
                .nth(1)
                .unwrap_or("")
                .split('`')
                .filter(|t| t.ends_with("_implements") || t.ends_with("_monotone"))
                .collect();
            if theorems.is_empty() && !row.contains("tested only") {
                return Err(format!("{word}: no theorem and not marked tested only"));
            }
            for t in theorems {
                if !claims.contains(&format!(".{t}\"")) {
                    return Err(format!("{word}: {t} is not a claimed theorem"));
                }
            }
        }
        for (module, class, _) in ALLOWED {
            if !register.contains(&format!("| `{module}` | {class} |")) {
                return Err(format!("{module} ({class}) is not in the register"));
            }
        }
        let listed = register
            .lines()
            .filter(|l| l.starts_with("| `crates/"))
            .count();
        if listed != ALLOWED.len() {
            return Err(format!(
                "the register lists {listed} modules, ALLOWED {}",
                ALLOWED.len()
            ));
        }
        Ok(())
    }
}
