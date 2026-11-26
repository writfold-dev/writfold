//! The checks of a delivery mode file ([`super::mode`]): its keys, its
//! built-in checks, its gates (candidate gates of the KB root) and its
//! attested names, each fault at its position.
use super::config::{BUILTIN, is_name};
use super::config_check::{list, named};
use super::mode::{MODES, Mode};
use super::mode_keys::{checks, closed, description, waivable};
use crate::defs::{Check, Spanned, Text};
use crate::gate::def;
use crate::gate::def_check::schema;
use std::path::Path;
use writfold_core::diag::Report;
use writfold_core::diag::suggest::{did_you_mean, suggest};

/// A mode's gates: candidate gates of the KB root.
fn gates(c: &mut Check<'_>, root: &Path) -> Option<Vec<def::Definition>> {
    let ids = list(c, "gates")?;
    if ids.is_empty() {
        return Some(Vec::new());
    }
    let (found, problems) = def::all(root, None);
    let candidates: Vec<&str> = found
        .iter()
        .filter(|d| d.on_candidate())
        .map(|d| d.id.as_str())
        .collect();
    let mut out = Some(Vec::new());
    for id in &ids {
        let file = format!("{}/{}.toml", def::DIR, id.str());
        let fault = match found.iter().find(|d| d.id == id.str()) {
            Some(d) if d.on_candidate() => {
                if let Some(o) = out.as_mut() {
                    o.push(d.clone());
                }
                continue;
            }
            Some(_) => (
                "ref.wrong-kind",
                format!(
                    "gate `{}` judges a repository; a mode's gates need subject = \"candidate\"",
                    id.str()
                ),
                "a repository gate",
                None,
            ),
            None if problems.iter().any(|(_, p)| p.path == file) => {
                let first = problems
                    .iter()
                    .find(|(_, p)| p.path == file)
                    .and_then(|(_, p)| p.report.diagnostics.first());
                let why = first.map_or_else(String::new, |d| format!(": {}", d.message));
                (
                    "ref.undefined",
                    format!("gate `{}` cannot be read{why}", id.str()),
                    "has errors",
                    Some(format!("`{file}` exists but has errors")),
                )
            }
            None => {
                let mode = MODES.contains(&id.str())
                    || root
                        .join(super::config::DIR)
                        .join(format!("{}.toml", id.str()))
                        .is_file();
                let note = mode.then(|| {
                    format!(
                        "{} is a delivery mode, and a mode cannot name a mode",
                        id.str()
                    )
                });
                (
                    "ref.undefined",
                    format!("unknown gate `{}`: define {file} in the KB root", id.str()),
                    "no such candidate gate",
                    note,
                )
            }
        };
        let (code, why, label, note) = fault;
        let d = c.error(code, Some(id.span), why);
        d.label(label);
        d.help = did_you_mean(&suggest(id.str(), &candidates), '`');
        match note {
            Some(n) if n.contains("a mode cannot name a mode") => d.help = Some(n),
            n => d.notes.extend(n),
        }
        out = None;
    }
    out
}

/// Each obligation name once, and attest names new and well formed.
fn names(
    c: &mut Check<'_>,
    checks: &[Spanned<Text>],
    attest: &[Spanned<Text>],
    gates: &[Spanned<Text>],
) -> bool {
    let mut ok = true;
    let mut seen: Vec<&str> = Vec::new();
    for item in checks.iter().chain(gates).chain(attest) {
        if seen.contains(&item.str()) {
            let why = format!("{} is listed twice", item.str());
            c.error("value.twice", Some(item.span), why)
                .label("listed again");
            ok = false;
        }
        seen.push(item.str());
    }
    for a in attest
        .iter()
        .filter(|a| !is_name(a.str()) || BUILTIN.contains(&a.str()))
    {
        let why = format!("attest name {:?} must be a new name of [a-z0-9.-]", a.str());
        c.error("value.form", Some(a.span), why)
            .label("not a new name");
        ok = false;
    }
    ok
}

/// Every key a mode file may have.
const KEYS: [&str; 16] = [
    "schema",
    "mode",
    "description",
    "checks",
    "paragraph_words",
    "max_chars",
    "max_lines",
    "gates",
    "attest",
    "waivable",
    "rendering",
    "audience",
    "prose",
    "vale",
    "vale_block",
    "agent",
];

/// `<mode>.toml`, over the mode's defaults.
pub fn mode(root: &Path, mut mode: Mode, path: &str, text: &str) -> Result<Mode, Report> {
    let mut c = Check::read(path, text, "a delivery mode");
    c.unknown_keys(&KEYS, &KEYS, "not a delivery mode key");
    schema(&mut c, "writfold-delivery-mode/1");
    let ok = named(
        &mut c,
        "mode",
        &mode.name.clone(),
        &format!("{}.toml", mode.name),
    );
    let checks = checks(&mut c);
    let description = description(&mut c);
    let words = c.int("paragraph_words", (1, 10_000), 150).or(Spanned {
        value: i64::try_from(mode.paragraph_words).unwrap_or(150),
        span: Default::default(),
    });
    let max_chars = bound(c.int("max_chars", (1, 1_000_000), 4000));
    let max_lines = bound(c.int("max_lines", (1, 100_000), 40));
    let gate_items = list(&mut c, "gates").unwrap_or_default();
    let gates = gates(&mut c, root);
    let attest = list(&mut c, "attest");
    let unique = names(
        &mut c,
        checks.clone().flatten().unwrap_or_default().as_slice(),
        attest.as_deref().unwrap_or(&[]),
        &gate_items,
    );
    if let Some(Some(checks)) = &checks {
        mode.checks = checks.iter().map(|x| x.value.text.clone()).collect();
    }
    let declared: Vec<String> = (mode.checks.iter())
        .chain(attest.iter().flatten().map(|a| &a.value.text))
        .cloned()
        .collect();
    let listed = |check: &str| mode.checks.iter().any(|x| x == check);
    let prose = super::mode_keys::prose(&mut c, root, listed(super::config::PROSE));
    let vale = super::vale_profile::key(&mut c, root, listed(super::config::VALE));
    let agent = super::mode_keys::agent(&mut c, listed(super::config::BOUND));
    let measured = (mode.checks.iter()).any(|x| x == super::config::LENGTH);
    let unbounded = matches!((&max_chars, &max_lines), (Some(None), Some(None)));
    if measured && unbounded {
        let why = "length.bounded needs max_chars or max_lines".to_owned();
        c.error("key.missing", None, why).label("no bound");
    }
    let gate_ids: Vec<&str> = gate_items.iter().map(Spanned::str).collect();
    let waivable = waivable(&mut c, &declared, &gate_ids, text);
    let rendering = closed(&mut c, "rendering", &super::render::RENDERINGS);
    let audience = closed(&mut c, "audience", &super::config::AUDIENCES)
        .map(|a| a.map(|a| (a, super::mode::key_line(text, "audience"))));
    let built = (|| {
        mode.description = description?;
        mode.paragraph_words = usize::try_from(words?.value).ok()?;
        mode.max_chars = max_chars?;
        mode.max_lines = max_lines?;
        mode.gates = gates?;
        mode.attest = attest?.into_iter().map(|a| a.value.text).collect();
        mode.waivable = waivable?;
        if let Some(r) = rendering? {
            mode.rendering = r;
        }
        mode.audience = audience?;
        mode.prose = prose?;
        mode.vale = vale?;
        mode.agent = agent?;
        mode.source = path.to_owned();
        (ok && unique && !(measured && unbounded)).then_some(mode)
    })();
    c.finish(built)
}

/// An optional bound: `Some(None)` when absent, `None` when reported bad.
fn bound(field: crate::defs::fields::Field<Spanned<i64>>) -> Option<Option<usize>> {
    match field {
        crate::defs::fields::Field::Absent => Some(None),
        crate::defs::fields::Field::Bad => None,
        crate::defs::fields::Field::Got(v) => usize::try_from(v.value).ok().map(Some),
    }
}
