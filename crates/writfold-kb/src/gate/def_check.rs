//! The checks of a gate definition ([`super::def`]). Every check runs and
//! reports at the position of what it judges; a check that needs a value
//! another check found wrong does not run, so no fault is reported twice.
use super::def::{
    CHECKLIST, CLASSES, DIR, Definition, Instructions, Judges, Origin, RUN_RECORD,
    SCHEMA, digest,
};
use super::{def_class, places};
use crate::defs::{Check, Field, Slot, Spanned, Text, Value};
use std::path::Path;
use writfold_core::diag::Report;

const KEYS: [&str; 21] = [
    "accept",
    "schema",
    "id",
    "version",
    "class",
    "argv",
    "workdir",
    "timeout_seconds",
    "scope",
    "description",
    "instructions",
    "subject",
    "require",
    "items",
    "optional",
    "not_applicable",
    "none_found",
    "snapshot",
    "own_evidence",
    "evidence_from",
    "ages_as",
];
const COMMON: [&str; 7] = [
    "accept",
    "schema",
    "id",
    "version",
    "class",
    "description",
    "instructions",
];

/// The keys a class takes, for suggestions.
fn keys_for(class: Option<&str>) -> Vec<&'static str> {
    let own: &[&str] = match class {
        Some("mechanical" | "observed") => &[
            "argv",
            "workdir",
            "timeout_seconds",
            "scope",
            "subject",
            "ages_as",
        ],
        Some("attested") => &["scope", "subject"],
        Some(RUN_RECORD) => &["require"],
        Some(CHECKLIST) => &[
            "scope",
            "items",
            "optional",
            "not_applicable",
            "none_found",
            "snapshot",
            "own_evidence",
            "evidence_from",
        ],
        _ => return KEYS.to_vec(),
    };
    COMMON.iter().chain(own).copied().collect()
}

/// `a mechanical gate`, `an attested gate`.
pub fn a_class(class: &str) -> String {
    match class.starts_with(['a', 'e', 'i', 'o', 'u']) {
        true => format!("an {class} gate"),
        false => format!("a {class} gate"),
    }
}

fn class(c: &mut Check<'_>) -> Option<String> {
    let class = c.need_text("class", "class = \"mechanical\"")?;
    if CLASSES.contains(&class.str()) {
        return Some(class.value.text);
    }
    let why = format!("unknown class {:?}", class.str());
    c.unknown_value(
        class.span,
        why,
        "not a gate class",
        class.str(),
        &CLASSES,
        '"',
    );
    None
}

/// `schema`, which every kind of file states exactly.
pub fn schema(c: &mut Check<'_>, want: &str) {
    let Some(schema) = c.need_text("schema", &format!("schema = \"{want}\"")) else {
        return;
    };
    if schema.str() != want {
        let why = format!("`schema` must be \"{want}\"");
        let d = c.error("value.unknown", Some(schema.span), why);
        d.label("not this file's schema");
        d.help(format!("write schema = \"{want}\""));
    }
}

/// An optional one-line `description`.
pub fn description(c: &mut Check<'_>) -> Option<Option<String>> {
    match c.text("description") {
        Field::Absent => Some(None),
        Field::Bad => None,
        Field::Got(d) if d.str().contains(['\n', '\r']) => {
            let why = "`description` must be one line".into();
            c.error("value.form", Some(d.span), why)
                .label("holds a line break");
            None
        }
        Field::Got(d) => Some(Some(d.value.text)),
    }
}

/// `subject`: `repository` (the default) or `candidate`. A candidate gate
/// lives in the KB root, where delivery modes look for it.
fn judges(c: &mut Check<'_>, origin: Origin) -> Option<Judges> {
    let subject = match c.text("subject") {
        Field::Absent => return Some(Judges::Repository),
        Field::Bad => return None,
        Field::Got(s) => s,
    };
    match subject.str() {
        "repository" => Some(Judges::Repository),
        "candidate" if origin == Origin::Kb => Some(Judges::Candidate),
        "candidate" => {
            let why = "a candidate gate belongs in the KB root's .writfold/gates/, where delivery modes read it".into();
            c.error("ref.wrong-kind", Some(subject.span), why)
                .label("in a repository");
            None
        }
        other => {
            let why = format!("unknown subject {other:?}");
            let set = ["repository", "candidate"];
            c.unknown_value(subject.span, why, "not a subject", other, &set, '"');
            None
        }
    }
}

/// A relative glob: not empty, not absolute, no backslash.
pub fn globs(c: &mut Check<'_>, patterns: &[Spanned<Text>], key: &str) -> bool {
    let mut ok = true;
    for p in patterns {
        let bare = p.str().strip_prefix('!').unwrap_or(p.str());
        if !super::repos::well_formed(p.str()) {
            let why = format!("{key} pattern {:?} must be repo:<store>/<glob>", p.str());
            let d = c.error("value.form", Some(p.span), why);
            d.label("not a store and a relative glob");
            d.help(
                "name a store declared in .writfold/stores/, such as \"repo:docs/adr/**\"".into(),
            );
            ok = false;
        } else if bare.is_empty() || bare.starts_with('/') || bare.contains('\\') {
            let why = format!("{key} pattern {:?} is not a relative glob", p.str());
            let d = c.error("value.form", Some(p.span), why);
            d.label("not a relative glob");
            d.help("write a path pattern relative to the repository, such as \"src/**\"".into());
            ok = false;
        }
    }
    ok
}

/// `scope`: required for a repository gate, refused for a candidate gate
/// (whose subject is the candidate's bytes) and an assessed gate.
fn scope(c: &mut Check<'_>, judges: Option<Judges>, class: &str) -> Option<Vec<String>> {
    if class == RUN_RECORD {
        return Some(Vec::new());
    }
    if judges == Some(Judges::Candidate) {
        let (who, why) = ("a candidate gate", "it judges the candidate's bytes");
        let ok = !c.not_here("scope", format!("`scope` is not used by {who}"), why);
        return ok.then(Vec::new);
    }
    judges?;
    let add = "scope = [\"src/**\"]";
    let patterns = c.need_list("scope", &a_class(class), add)?;
    globs(c, &patterns, "scope").then(|| patterns.into_iter().map(|p| p.value.text).collect())
}

/// `instructions`: a Markdown file inside the definition's directory.
fn instructions(c: &mut Check<'_>, dir: &Path) -> Option<Option<Instructions>> {
    let path = match c.text("instructions") {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(p) => p,
    };
    let fault = match path.str().ends_with(".md") {
        false => Err(format!(
            "instructions {:?} must name a .md file",
            path.str()
        )),
        true => places::resolve(dir, path.str())
            .and_then(|real| places::digest_file(&real))
            .map_err(|why| format!("instructions: {why}")),
    };
    match fault {
        Ok(digest) => Some(Some(Instructions {
            path: path.value.text,
            digest,
        })),
        Err(why) => {
            c.error("value.form", Some(path.span), why)
                .label("not a Markdown file beside the definition");
            None
        }
    }
}

/// Parse and check one definition; every fault is in the report.
pub fn check(
    text: &str,
    stem: &str,
    origin: Origin,
    dir: &Path,
    file: &str,
) -> Result<Definition, Report> {
    let mut c = Check::read(file, text, "a gate definition");
    let written = match c.doc.get("class").map(|e| &e.value) {
        Some(Slot::Value(v)) => match &v.value {
            Value::Str(t) => Some(t.text.clone()),
            _ => None,
        },
        _ => None,
    };
    c.unknown_keys(&KEYS, &keys_for(written.as_deref()), "not a gate key");
    let class = class(&mut c);
    schema(&mut c, SCHEMA);
    let id = c.need_text("id", &format!("id = \"{stem}\""));
    let id = id.filter(|id| c.named("id", id, stem));
    let version = c.need_int("version", (1, i64::from(u32::MAX)), 1, "version = 1");
    let description = description(&mut c);
    let instructions = instructions(&mut c, dir);
    let Some(class) = class else {
        return c.finish(None);
    };
    let require = def_class::require(&mut c, &class);
    let alone = def_class::checklist_subject(&mut c, &class);
    let judges = judges(&mut c, origin);
    let command = def_class::command(&mut c, &class, judges);
    let scope = scope(&mut c, judges, &class);
    let checklist = super::checklist::spec::check(&mut c, &class);
    let accept = super::def_accept::check(&mut c, &class);
    let ages_as = def_class::ages_as(&mut c, &class);
    let files = command
        .as_ref()
        .and_then(|(argv, ..)| def_class::files(&mut c, argv, dir));
    let built = (|| {
        let (argv, workdir, timeout_seconds) = command?;
        alone?;
        let id = id?.value.text;
        Some(Definition {
            path: format!("{DIR}/{id}.toml"),
            id,
            version: version?.value,
            class,
            argv: argv.into_iter().map(|a| a.value.text).collect(),
            workdir,
            timeout_seconds,
            scope: scope?,
            description: description?,
            origin,
            judges: judges?,
            instructions: instructions?,
            require: require?,
            files: files?,
            checklist: checklist?,
            accept: accept?,
            ages_as: ages_as?,
            dir: dir.to_path_buf(),
            digest: String::new(),
        })
    })();
    let built = built.map(|mut d| {
        d.digest = digest(&d);
        d
    });
    c.finish(built)
}
