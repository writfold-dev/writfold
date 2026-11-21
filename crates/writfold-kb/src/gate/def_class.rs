//! The per-class rules of a gate definition ([`super::def`]): which
//! command fields a class takes, a run-record gate's `require`, a
//! checklist gate's subject, and the placeholders a command may use.
use super::def::{ATTESTED, CHECKLIST, Judges, MAX_TIMEOUT, RUN_RECORD};
use super::def_check::a_class;
use super::places;
use crate::defs::{Check, Spanned, Text};
use std::path::Path;

/// The line a missing `timeout_seconds` is added as.
const TIMEOUT: &str = "timeout_seconds = 600   (seconds, 1 to 86400)";

/// Check each argument's placeholders against what the gate judges: a
/// repository gate may not use `{candidate}`; a candidate gate must use it
/// and may not use `{artifacts}` (its receipt keeps no local files).
fn placeholders(
    c: &mut Check<'_>,
    argv: &Spanned<Vec<Spanned<Text>>>,
    judges: Option<Judges>,
) -> bool {
    let mut ok = true;
    let mut all = Vec::new();
    for arg in &argv.value {
        let fault = match arg.str().is_empty() || arg.str().contains('\0') {
            true => Err("argv entries must be non-empty".to_owned()),
            false => places::used(arg.str()),
        };
        match fault {
            Ok(used) => all.extend(used.into_iter().map(|p| (p, arg))),
            Err(why) => {
                c.error("value.form", Some(arg.span), why)
                    .label("not a usable argument");
                ok = false;
            }
        }
    }
    let at = |p: &str| {
        all.iter()
            .find(|(u, _)| *u == p)
            .map(|(_, arg)| arg.sub(arg.str().find(p).unwrap_or(0), p.len()))
    };
    let fault = match judges {
        Some(Judges::Repository) => (at(places::CANDIDATE).map(|s| (s, places::CANDIDATE)))
            .or_else(|| at(places::SOURCE).map(|s| (s, places::SOURCE)))
            .map(|(s, p)| {
                (
                    Some(s),
                    format!("{p} is for gates with subject = \"candidate\""),
                )
            }),
        Some(Judges::Candidate) if at(places::CANDIDATE).is_none() => Some((
            Some(argv.span),
            format!(
                "a candidate gate must pass {} to its command",
                places::CANDIDATE
            ),
        )),
        Some(Judges::Candidate) => at(places::ARTIFACTS).map(|s| {
            (
                Some(s),
                format!("{} is for repository gates", places::ARTIFACTS),
            )
        }),
        None => None,
    };
    if let Some((span, why)) = fault {
        c.error("value.form", span, why)
            .label("not for this gate's subject");
        ok = false;
    }
    ok
}

/// The command fields: `argv`, `workdir` and `timeout_seconds`, required
/// for a check and refused for an attested or checklist gate (which runs
/// nothing; it is met only by `gate attest` or `gate cover`).
pub(super) fn command(
    c: &mut Check<'_>,
    class: &str,
    judges: Option<Judges>,
) -> Option<(Vec<Spanned<Text>>, String, u64)> {
    let gate = a_class(class);
    if [ATTESTED, RUN_RECORD, CHECKLIST].contains(&class) {
        let runs = match class {
            ATTESTED => "an attested gate runs nothing",
            CHECKLIST => "a checklist gate runs nothing (gate cover records its items)",
            _ => "a run-record gate runs no command",
        };
        let mut ok = true;
        for key in ["argv", "workdir", "timeout_seconds"] {
            if class == ATTESTED || class == CHECKLIST || key != "timeout_seconds" {
                ok &= !c.not_here(key, format!("`{key}` is not used by {gate}"), runs);
            }
        }
        return ok.then(|| (Vec::new(), ".".into(), 0));
    }
    let argv = c.list("argv");
    let argv = match argv {
        crate::defs::Field::Got(a) if a.value.is_empty() => {
            c.need_list("argv", &gate, "");
            None
        }
        crate::defs::Field::Got(a) => Some(a),
        crate::defs::Field::Absent => {
            c.missing("argv", &gate, "argv = [\"scripts/check\"]");
            None
        }
        crate::defs::Field::Bad => None,
    };
    let fine = argv.as_ref().map(|a| placeholders(c, a, judges));
    let workdir = c.text("workdir").got();
    let workdir_ok = match &workdir {
        Some(w) if w.str() != "." && !places::contained(w.str()) => {
            let why = "workdir must be a relative path inside the repository".into();
            c.error("value.form", Some(w.span), why)
                .label("not inside the repository");
            false
        }
        _ => c.doc.get("workdir").is_none() || workdir.is_some(),
    };
    let timeout = timeout(c, &gate);
    let (argv, timeout) = (argv?, timeout?);
    (fine? && workdir_ok).then_some(())?;
    let workdir = workdir.map_or_else(|| ".".into(), |w| w.value.text);
    Some((argv.value, workdir, timeout))
}

fn timeout(c: &mut Check<'_>, gate: &str) -> Option<u64> {
    let n = match c.int("timeout_seconds", (1, MAX_TIMEOUT), 600) {
        crate::defs::Field::Absent => {
            c.missing("timeout_seconds", gate, TIMEOUT);
            None
        }
        other => other.got(),
    }?;
    u64::try_from(n.value).ok()
}

/// The files `argv` names through `{gate}/…`, with their digests.
pub(super) fn files(
    c: &mut Check<'_>,
    argv: &[Spanned<Text>],
    dir: &Path,
) -> Option<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut ok = true;
    for arg in argv {
        match places::gate_files(std::slice::from_ref(&arg.value.text), dir) {
            Ok(found) => out.extend(
                found
                    .into_iter()
                    .filter(|f| !out.iter().any(|(p, _)| *p == f.0))
                    .collect::<Vec<_>>(),
            ),
            Err(why) => {
                c.error("value.form", Some(arg.span), why)
                    .label("not a file beside the definition");
                ok = false;
            }
        }
    }
    ok.then_some(out)
}

/// `require`: the built-in requirements of a run-record gate, which judges
/// the run's own records (so it takes no `scope` or `subject`); refused on
/// every other class.
pub(super) fn require(c: &mut Check<'_>, class: &str) -> Option<Vec<String>> {
    if class != RUN_RECORD {
        let why = format!("`require` is for class = \"{RUN_RECORD}\"");
        return (!c.not_here("require", why, "only a run-record gate takes it")).then(Vec::new);
    }
    let mut ok = true;
    for key in ["scope", "subject"] {
        let why = format!("`{key}` is not used by a run-record gate");
        ok &= !c.not_here(key, why, "a run-record gate judges the run's own records");
    }
    let items = c.need_list(
        "require",
        "a run-record gate",
        "require = [\"run.reflected\"]",
    )?;
    let known = crate::plan::records::REQUIREMENTS;
    for (n, item) in items.iter().enumerate() {
        if !known.contains(&item.str()) {
            let why = format!("unknown requirement {:?}", item.str());
            c.unknown_value(
                item.span,
                why,
                "not a built-in requirement",
                item.str(),
                &known,
                '"',
            );
            ok = false;
        } else if items[..n].iter().any(|i| i.str() == item.str()) {
            let why = format!("`require` names {} twice", item.str());
            c.error("value.twice", Some(item.span), why)
                .label("named again");
            ok = false;
        }
    }
    ok.then(|| items.into_iter().map(|i| i.value.text).collect())
}

/// A checklist gate judges a repository's content in its scope: it takes
/// no `subject` (a candidate has no cover rows).
pub(super) fn checklist_subject(c: &mut Check<'_>, class: &str) -> Option<()> {
    let why = "`subject` is not used by a checklist gate".to_owned();
    let label = "a checklist gate judges the repository content in its scope";
    match class == CHECKLIST && c.not_here("subject", why, label) {
        true => None,
        false => Some(()),
    }
}

/// `ages_as = "KIND"` on a mechanical or observed gate (gate-evidence
/// spec, stage 5): its receipts age by that kind's lifetime in the catalog
/// (`.writfold/sources/KIND.toml`). `Some(None)` when absent.
pub fn ages_as(c: &mut Check<'_>, class: &str) -> Option<Option<String>> {
    let candidate = matches!(c.doc.get("subject").map(|e| &e.value),
        Some(crate::defs::Slot::Value(v)) if matches!(&v.value, crate::defs::Value::Str(t) if t.text == "candidate"));
    if candidate {
        let why = "`ages_as` is not taken by candidate gates (delivery checks)".to_owned();
        return (!c.not_here("ages_as", why, "a repository gate only")).then_some(None);
    }
    if !matches!(class, "mechanical" | "observed") {
        let why = "`ages_as` is for mechanical and observed gates".to_owned();
        return (!c.not_here("ages_as", why, "only a command gate takes it")).then_some(None);
    }
    match c.text("ages_as") {
        crate::defs::Field::Absent => Some(None),
        crate::defs::Field::Bad => None,
        crate::defs::Field::Got(t) if crate::stale::kinds::is_word(t.str()) => {
            Some(Some(t.value.text))
        }
        crate::defs::Field::Got(t) => {
            let why = format!("ages_as {:?} must be a kind name ([a-z0-9._-])", t.str());
            c.error("value.form", Some(t.span), why)
                .label("not a kind name");
            None
        }
    }
}
