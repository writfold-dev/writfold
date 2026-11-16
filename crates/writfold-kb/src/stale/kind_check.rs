//! The checks of a kind declaration ([`super::kinds`]): every fault of
//! the file, each at its position.
use super::kinds::{BUILT_IN, Freshness, Kind, OWNED, SCHEMA, is_word};
use crate::defs::{Check, Field, Spanned, Text};
use crate::gate::def_check::{description, schema};
use crate::time;
use writfold_core::diag::Report;

const KEYS: [&str; 9] = [
    "schema",
    "kind",
    "freshness",
    "default_ttl",
    "required",
    "description",
    "checker",
    "fields",
    "max_age",
];
const FRESHNESS: [&str; 3] = ["ttl", "immutable", "checked"];

fn kind(c: &mut Check<'_>, name: &str) -> Option<String> {
    let kind = c.need_text("kind", &format!("kind = \"{name}\""))?;
    if !c.named("kind", &kind, name) {
        return None;
    }
    if !is_word(kind.str()) {
        let why = format!("kind {:?} must be [a-z0-9._-]", kind.str());
        c.error("value.form", Some(kind.span), why)
            .label("not a kind name");
        return None;
    }
    if BUILT_IN.contains(&kind.str()) {
        let why = format!(
            "kind {} is checked against Git and the index and cannot be declared",
            kind.str()
        );
        c.error("value.unknown", Some(kind.span), why)
            .label("a built-in kind");
        return None;
    }
    Some(kind.value.text)
}

fn freshness(c: &mut Check<'_>) -> Option<Freshness> {
    let f = c.need_text("freshness", "freshness = \"ttl\"")?;
    match f.str() {
        "ttl" => Some(Freshness::Ttl),
        "immutable" => Some(Freshness::Immutable),
        "checked" => Some(Freshness::Checked),
        other => {
            let why = format!("unknown freshness {other:?}");
            c.unknown_value(f.span, why, "not a freshness", other, &FRESHNESS, '"');
            None
        }
    }
}

/// A duration `<N>d` or `<N>h`, which `freshness` must allow.
fn duration(
    c: &mut Check<'_>,
    key: &str,
    freshness: Option<Freshness>,
    wants: Freshness,
) -> Option<Option<String>> {
    let v = match c.text(key) {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(v) => v,
    };
    let fits = match wants {
        Freshness::Ttl => freshness == Some(Freshness::Ttl),
        _ => freshness == Some(Freshness::Checked),
    };
    if freshness.is_some() && !fits {
        let f = freshness.map_or("", |f| match f {
            Freshness::Ttl => "ttl",
            Freshness::Immutable => "immutable",
            Freshness::Checked => "checked",
        });
        let why = match key {
            "max_age" => "max_age needs freshness = \"checked\"".to_owned(),
            _ => format!("default_ttl cannot be given with freshness = \"{f}\""),
        };
        c.error("key.not-here", c.key_span(key), why)
            .help("remove this line".into());
        return None;
    }
    if !matches!(time::ttl(Some(v.str())), Some(time::Ttl::Seconds(_))) {
        let why = format!("{key} {:?} must be <N>d or <N>h", v.str());
        c.error("value.form", Some(v.span), why)
            .label("not a duration")
            .help(format!("write {key} = \"30d\""));
        return None;
    }
    Some(Some(v.value.text))
}

/// An optional array of field names.
fn names(c: &mut Check<'_>, key: &str) -> Option<Vec<Spanned<Text>>> {
    let list = c
        .list(key)
        .or(Spanned {
            value: Vec::new(),
            span: Default::default(),
        })?
        .value;
    let mut ok = true;
    for f in list.iter().filter(|f| !is_word(f.str()) && f.str() != "L") {
        let why = format!("{key} field {:?} is not a field name", f.str());
        c.error("value.form", Some(f.span), why)
            .label("not a field name");
        ok = false;
    }
    ok.then_some(list)
}

/// `checker` and `fields`: both required with `freshness = "checked"`,
/// refused otherwise.
fn checked(
    c: &mut Check<'_>,
    freshness: Option<Freshness>,
) -> Option<(Option<String>, Vec<String>)> {
    let checker = c.text("checker");
    let fields = names(c, "fields");
    let freshness = freshness?;
    if freshness != Freshness::Checked {
        let mut ok = true;
        for key in ["checker", "fields"] {
            let why = format!(
                "`{key}` is used only with freshness = \"checked\"; checker and fields need freshness = \"checked\""
            );
            ok &= !c.not_here(key, why, "this kind is not checked");
        }
        let fields = fields?.into_iter().map(|f| f.value.text).collect();
        return ok.then_some((None, fields));
    }
    let checker = match checker {
        Field::Absent => {
            c.missing(
                "checker",
                "freshness = \"checked\"",
                "checker = \"<name>\"   (freshness = \"checked\" needs checker)",
            );
            None
        }
        other => other.got(),
    };
    let checker = checker.filter(|ch| {
        let ok = is_word(ch.str());
        if !ok {
            let why = format!("checker {:?} must be a name [a-z0-9._-]", ch.str());
            c.error("value.form", Some(ch.span), why)
                .label("not a name");
        }
        ok
    });
    let fields = fields?;
    if fields.is_empty() {
        c.missing(
            "fields",
            "freshness = \"checked\"",
            "fields = [\"id\"]   (freshness = \"checked\" needs fields)",
        );
        return None;
    }
    if let Some(f) = fields.iter().find(|f| OWNED.contains(&f.str())) {
        let why = format!("fields may not list {}, which Writfold writes", f.str());
        c.error("value.form", Some(f.span), why)
            .label("written by Writfold");
        return None;
    }
    Some((
        Some(checker?.value.text),
        fields.into_iter().map(|f| f.value.text).collect(),
    ))
}

/// Every fault of one declaration, or the declaration.
pub fn check(text: &str, name: &str, shown: &str) -> Result<Kind, Report> {
    let mut c = Check::read(shown, text, "a kind declaration");
    c.unknown_keys(&KEYS, &KEYS, "not a kind declaration key");
    schema(&mut c, SCHEMA);
    let kind = kind(&mut c, name);
    let freshness = freshness(&mut c);
    let default_ttl = duration(&mut c, "default_ttl", freshness, Freshness::Ttl);
    let required = names(&mut c, "required");
    let checked = checked(&mut c, freshness);
    let max_age = duration(&mut c, "max_age", freshness, Freshness::Checked);
    let description = description(&mut c);
    let built = (|| {
        let (checker, fields) = checked?;
        Some(Kind {
            name: kind?,
            freshness: freshness?,
            default_ttl: default_ttl?,
            required: required?.into_iter().map(|f| f.value.text).collect(),
            description: description?,
            checker,
            fields,
            max_age: max_age?,
        })
    })();
    c.finish(built)
}
