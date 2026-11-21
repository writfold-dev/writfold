//! The checks of a store declaration ([`super`]): every fault of the
//! file, each at its position.
use super::{Freshness, SCHEMA, Store};
use crate::defs::{Check, Field};
use crate::gate::def_check::{description, schema};
use crate::stale::kinds::is_word;
use writfold_core::diag::Report;

const KEYS: [&str; 8] = [
    "schema",
    "name",
    "url",
    "role",
    "freshness",
    "paths",
    "description",
    "recorded_max_age",
];
const ROLES: [&str; 1] = ["attached"];
const FRESHNESS: [&str; 2] = ["pinned", "head"];

fn name(c: &mut Check<'_>, file: &str) -> Option<String> {
    let name = c.need_text("name", &format!("name = \"{file}\""))?;
    if !c.named("name", &name, file) {
        return None;
    }
    if !is_word(name.str()) {
        let why = format!("name {:?} must be [a-z0-9._-]", name.str());
        c.error("value.form", Some(name.span), why)
            .label("not a store name");
        return None;
    }
    Some(name.value.text)
}

fn url(c: &mut Check<'_>) -> Option<String> {
    let url = c.need_text("url", "url = \"<the remote every machine clones from>\"")?;
    if url.str().trim().is_empty() || url.str().contains(['\n', '\r', ' ']) {
        let why = "`url` must be one remote URL with no spaces".to_owned();
        c.error("value.form", Some(url.span), why)
            .label("not a remote URL");
        return None;
    }
    Some(url.value.text)
}

fn role(c: &mut Check<'_>) -> Option<()> {
    let role = c.need_text("role", "role = \"attached\"")?;
    if role.str() != "attached" {
        let why = format!("unknown role {:?}", role.str());
        c.unknown_value(role.span, why, "not a role", role.str(), &ROLES, '"');
        return None;
    }
    Some(())
}

fn freshness(c: &mut Check<'_>) -> Option<Freshness> {
    match c.text("freshness") {
        Field::Absent => Some(Freshness::Pinned),
        Field::Bad => None,
        Field::Got(f) => match f.str() {
            "pinned" => Some(Freshness::Pinned),
            "head" => Some(Freshness::Head),
            other => {
                let why = format!("unknown freshness {other:?}");
                c.unknown_value(f.span, why, "not a freshness", other, &FRESHNESS, '"');
                None
            }
        },
    }
}

fn paths(c: &mut Check<'_>) -> Option<Vec<String>> {
    let empty = crate::defs::Spanned {
        value: Vec::new(),
        span: Default::default(),
    };
    let list = c.list("paths").or(empty)?.value;
    let mut ok = true;
    for p in list
        .iter()
        .filter(|p| p.str().trim().is_empty() || p.str().contains('\n'))
    {
        let why = format!("path hint {:?} is not a path", p.str());
        c.error("value.form", Some(p.span), why).label("not a path");
        ok = false;
    }
    ok.then(|| list.into_iter().map(|p| p.value.text).collect())
}

/// `recorded_max_age`: how old a recorded verdict of a pinned citation a
/// reader without the clone may use (#4), `<N>d` or `<N>h`; only beside
/// `freshness = "pinned"`.
fn recorded_max_age(c: &mut Check<'_>, freshness: Option<Freshness>) -> Option<Option<String>> {
    let v = match c.text("recorded_max_age") {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(v) => v,
    };
    if freshness == Some(Freshness::Head) {
        let why = "recorded_max_age needs freshness = \"pinned\" (a head-judged store never uses a receipt)";
        c.error(
            "key.not-here",
            c.key_span("recorded_max_age"),
            why.to_owned(),
        )
        .help("remove this line".into());
        return None;
    }
    if !matches!(
        crate::time::ttl(Some(v.str())),
        Some(crate::time::Ttl::Seconds(_))
    ) {
        let why = format!("recorded_max_age {:?} must be <N>d or <N>h", v.str());
        c.error("value.form", Some(v.span), why)
            .label("not a duration")
            .help("write recorded_max_age = \"30d\"".into());
        return None;
    }
    Some(Some(v.value.text))
}

/// Every fault of one declaration, or the declaration.
pub fn check(text: &str, file: &str, shown: &str) -> Result<Store, Report> {
    let mut c = Check::read(shown, text, "a store declaration");
    c.unknown_keys(&KEYS, &KEYS, "not a store declaration key");
    schema(&mut c, SCHEMA);
    let name = name(&mut c, file);
    let url = url(&mut c);
    let role = role(&mut c);
    let freshness = freshness(&mut c);
    let paths = paths(&mut c);
    let description = description(&mut c);
    let recorded = recorded_max_age(&mut c, freshness);
    let built = (|| {
        role?;
        Some(Store {
            name: name?,
            url: url?,
            freshness: freshness?,
            paths: paths?,
            description: description?,
            recorded_max_age: recorded?,
        })
    })();
    c.finish(built)
}
