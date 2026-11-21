//! A catalog declaration (`.writfold/catalogs/<name>.toml`, #86): where a
//! project's asset inventory is and how to read it. Every fault of the
//! file, each at its position, for `kb lint` and the catalog verbs. The
//! file is the shared TOML subset plus one `[fields]` table: each half is
//! read by the same reader, the other half blanked so positions hold.
//! Tested, not proved: parsing.
use super::SCHEMA;
use crate::defs::{Check, Field};
use crate::gate::def_check::schema;
use writfold_core::diag::Report;

const KEYS: [&str; 3] = ["schema", "input", "items"];

/// The mapped fields: name, and whether it is a list of strings.
pub const FIELDS: [(&str, bool); 11] = [
    ("id", false),
    ("name", false),
    ("kind", false),
    ("owner", false),
    ("lifecycle", false),
    ("locator", false),
    ("verified_at", false),
    ("ttl", false),
    ("notes", false),
    ("aliases", true),
    ("projects", true),
];

/// One checked declaration.
#[derive(Clone, Debug)]
pub struct Decl {
    /// The file stem.
    pub name: String,
    /// KB-relative, as written.
    pub input: String,
    /// The pointer of the array in a single input file, if any.
    pub items: Option<String>,
    /// Field name and its RFC 6901 pointer, in [`FIELDS`] order.
    pub fields: Vec<(&'static str, String)>,
}

/// The text with every line outside `keep` (a line range) blanked.
fn blanked(text: &str, keep: std::ops::Range<usize>) -> String {
    let mut out = String::with_capacity(text.len());
    for (n, line) in text.split_inclusive('\n').enumerate() {
        if keep.contains(&n) {
            out.push_str(line);
        } else if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn pointer(c: &mut Check<'_>, key: &str, value: &crate::defs::Spanned<crate::defs::Text>) -> bool {
    let p = value.str();
    if p.is_empty() || p.starts_with('/') {
        return true;
    }
    let why = format!("`{key}` must be a JSON Pointer (RFC 6901): empty or starting with /");
    c.error("value.form", Some(value.span), why)
        .label("not a JSON Pointer")
        .help(format!("write {key} = \"/{p}\""));
    false
}

fn top(c: &mut Check<'_>, has_fields: bool) -> Option<(String, Option<String>)> {
    c.unknown_keys(&KEYS, &KEYS, "not a catalog declaration key");
    schema(c, SCHEMA);
    let input = c.need_text("input", "input = \".writfold/catalog-inputs/assets.json\"")?;
    let items = match c.text("items") {
        Field::Absent => None,
        Field::Bad => return None,
        Field::Got(i) => Some(pointer(c, "items", &i).then_some(i.value.text)?),
    };
    if !has_fields {
        c.error(
            "key.missing",
            None,
            "a catalog maps its fields in a [fields] table".into(),
        )
        .help("add [fields] with id = \"/id\"".into());
        return None;
    }
    Some((input.value.text, items))
}

fn fields(c: &mut Check<'_>) -> Option<Vec<(&'static str, String)>> {
    let names: Vec<&str> = FIELDS.iter().map(|(n, _)| *n).collect();
    c.unknown_keys(&names, &names, "not a mapped field");
    let mut out = Vec::new();
    let mut ok = true;
    for (name, _) in FIELDS {
        let got = match name {
            "id" => c
                .need_text("id", "id = \"/id\"")
                .map(Field::Got)
                .unwrap_or(Field::Bad),
            _ => c.text(name),
        };
        match got {
            Field::Absent => {}
            Field::Bad => ok = false,
            Field::Got(p) if pointer(c, name, &p) => out.push((name, p.value.text)),
            Field::Got(_) => ok = false,
        }
    }
    ok.then_some(out)
}

/// Every fault of one declaration named `stem`, or the declaration.
pub fn check(text: &str, stem: &str, shown: &str) -> Result<Decl, Report> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let header = lines.iter().position(|l| l.trim() == "[fields]");
    let split = header.unwrap_or(lines.len());
    let head_text = blanked(text, 0..split);
    let rest_text = blanked(text, split + 1..lines.len());
    let mut head = Check::read(shown, &head_text, "a catalog declaration");
    let named = crate::write::is_slug(stem);
    if !named {
        let why = format!("a catalog's file name {stem:?} must be a slug ([a-z0-9-])");
        head.error("value.form", None, why);
    }
    let first = top(&mut head, header.is_some());
    let mut rest = Check::read(shown, &rest_text, "a catalog declaration");
    let mapped = header.and_then(|_| fields(&mut rest));
    let decl = (|| {
        let (input, items) = first?;
        named.then_some(())?;
        Some(Decl {
            name: stem.to_owned(),
            input,
            items,
            fields: mapped?,
        })
    })();
    match (head.finish(decl), rest.finish(Some(()))) {
        (Ok(decl), Ok(())) => Ok(decl),
        (head, rest) => {
            let mut report = head.err().unwrap_or_default();
            if let Err(more) = rest {
                report.diagnostics.extend(more.diagnostics);
            }
            report.sources = vec![writfold_core::diag::Source {
                file: shown.to_owned(),
                text: text.to_owned(),
            }];
            Err(report)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    const GOOD: &str = "schema = \"writfold-asset-catalog/1\"\ninput = \".writfold/catalog-inputs/equipment.json\"\nitems = \"/equipment\"\n\n[fields]\nid = \"/tag\"\nname = \"/label\"\naliases = \"/search/names\"\n";

    #[test]
    fn a_declaration_and_its_faults() {
        let decl = check(GOOD, "brewery", "x.toml");
        assert!(
            matches!(&decl, Ok(d) if d.items.as_deref() == Some("/equipment") && d.fields.len() == 3),
            "{decl:?}"
        );
        let faults = [
            (GOOD.replace("id = \"/tag\"\n", ""), "id"),
            (GOOD.replace("[fields]", "[other]"), "table"),
            (GOOD.replace("label", "label\"\nnope = \"/x"), "nope"),
            (GOOD.replace("\"/tag\"", "\"tag\""), "JSON Pointer"),
            (GOOD.replace("asset-catalog/1", "asset-catalog/2"), "schema"),
            (
                format!("{GOOD}[fields]\nid = \"/x\"\n"),
                "tables are not used",
            ),
        ];
        for (text, word) in faults {
            let got = check(&text, "brewery", "x.toml");
            let why = got
                .as_ref()
                .err()
                .map(crate::defs::Check::why)
                .unwrap_or_default();
            assert!(got.is_err(), "{word}: {text}");
            assert!(why.contains(word) || word == "table", "{word}: {why}");
        }
        assert!(check(GOOD, "Bad Name", "x.toml").is_err());
    }
}
