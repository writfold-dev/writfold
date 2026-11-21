//! `kb policy` (policy spec 2.5): the policy as it is now, read from the
//! definition files of the KB root and the repository in view, and the
//! current run's pin beside it. Read-only; no command or checker is run.
//! `identity` is the SHA-256 over the sorted `<gate>\t<definition digest>`
//! lines, each declared store's rule, and the required file's digest: one
//! value two clones can compare.
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::gate::def::Definition;
use crate::guard::rules::{Event, Require};
use crate::session::Session;
use sha2::{Digest, Sha256};
use std::path::Path;

fn more(label: &'static str, text: String, path: Option<String>) -> More {
    More {
        label,
        text,
        line: path.as_ref().map(|_| 1),
        path,
        ..More::default()
    }
}

fn short(d: &str) -> &str {
    let d = d.strip_prefix("sha256:").unwrap_or(d);
    d.get(..12).unwrap_or(d)
}

/// `items=3 (precedents: supported|none-found; …)`.
fn items(d: &Definition) -> String {
    let Some(spec) = &d.checklist else {
        return String::new();
    };
    let each: Vec<String> = (spec.items.iter())
        .map(|i| {
            let mut words = vec!["supported"];
            if i.none {
                words.push("none-found");
            }
            if i.na {
                words.push("not-applicable");
            }
            format!("{}: {}", i.id, words.join("|"))
        })
        .collect();
    format!(" items={} ({})", spec.items.len(), each.join("; "))
}

fn gate_row(d: &Definition, pin: Option<More>) -> Row {
    let accept = d
        .accept
        .as_ref()
        .map_or_else(String::new, |a| format!(" accept={}", a.join(",")));
    let scope = match d.scope.is_empty() {
        true => String::new(),
        false => format!(" scope={}", d.scope.join(",")),
    };
    let mut row = Row {
        kind: "gate",
        id: d.id.clone(),
        text: format!(
            "{}{accept}{}{scope} @{}",
            d.class,
            items(d),
            short(&d.digest)
        ),
        path: Some(d.path.clone()),
        line: Some(1),
        section: Some("policy"),
        ..Row::default()
    };
    if let Some(i) = &d.instructions {
        let at = Path::new(&d.path).parent().map(|p| p.join(&i.path));
        let at = at.map(|p| p.to_string_lossy().into_owned());
        row.more.push(more("read", "instructions".into(), at));
    }
    row.more.extend(pin);
    row
}

fn guard_rows(root: &Path, cwd: &Path) -> Vec<Row> {
    let (rules, _) = crate::guard::rules::all(root, cwd);
    (rules.iter())
        .map(|r| {
            let event = match r.event {
                Event::PreToolUse => "pre-tool-use",
                Event::Stop => "stop",
            };
            let what: Vec<String> = (r.commands.iter().map(|c| c.join(" ")))
                .chain(r.tools.iter().cloned())
                .collect();
            let paths = match r.paths.is_empty() {
                true => String::new(),
                false => format!(" paths={}", r.paths.join(",")),
            };
            let require: Vec<String> = (r.require.iter())
                .map(|q| match q {
                    Require::RunCurrent => "run.current".into(),
                    Require::GatesMet => "gates.met".into(),
                    Require::Gate(g) => format!("gate:{g}"),
                    Require::StepMet => "step.met".into(),
                })
                .collect();
            Row {
                kind: "guard",
                id: r.id.clone(),
                text: format!(
                    "{event} {}{paths} requires {} (a guard, not enforcement)",
                    what.join(","),
                    require.join(" ")
                ),
                path: Some(r.path.clone()),
                line: Some(1),
                section: Some("policy"),
                ..Row::default()
            }
        })
        .collect()
}

fn count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
        .count()
}

/// The `kb policy` answer.
pub fn policy(session: &Session<'_>, cwd: &Path) -> Result<Answer> {
    let root = &session.ws.root;
    let top = crate::defs::lint::enclosing(root, cwd);
    let mut answer = Answer::default();
    if !crate::gate::policy::configured(root, top.as_deref()) {
        answer.unknown = Some("no-policy");
        answer
            .notes
            .push(crate::gate::policy::NONE_CONFIGURED.into());
        return Ok(answer);
    }
    let repo = top.as_deref().map(|t| (t, ""));
    let required = crate::plan::required::find(root, repo).unwrap_or_default();
    let pin = super::policy_pin::current(session, cwd);
    if let Some(source) = &required.source {
        let text = format!(
            "start={} end={} unskippable={}",
            required.start.join(","),
            required.end.join(","),
            required.unskippable.join(",")
        );
        let path = Some(source.clone());
        let row = Row {
            kind: "required",
            id: "required".into(),
            text,
            path,
            line: Some(1),
            section: Some("policy"),
            ..Row::default()
        };
        answer.rows.push(row);
    }
    let (defs, _) = crate::gate::def::all(root, top.as_deref());
    let mut lines: Vec<String> = Vec::new();
    for d in &defs {
        lines.push(format!("{}\t{}", d.id, d.digest));
        answer.rows.push(gate_row(d, pin.line(&d.id, &d.digest)));
    }
    answer.rows.extend(guard_rows(root, cwd));
    // The delivery modes, as `deliver modes` shows them, and the guard's routes.
    let modes = crate::deliver::modes::names(root);
    answer
        .rows
        .extend(modes.iter().map(|m| crate::deliver::modes::row(root, m)));
    answer.rows.extend(route_rows(root));
    let stores = store_lines(root, &mut lines)?;
    lines.sort();
    let mut material = lines.join("\n");
    material.push('\n');
    material.push_str(required.digest.as_deref().unwrap_or("absent"));
    let identity = format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(material.as_bytes()))
    );
    let dirs = |d: &str| {
        count(&root.join(".writfold").join(d))
            + top
                .as_ref()
                .map_or(0, |t| count(&t.join(".writfold").join(d)))
    };
    answer.counts = vec![
        ("gates", defs.len()),
        (
            "templates",
            dirs("plans").saturating_sub(usize::from(required.source.is_some())),
        ),
        ("guards", dirs("guards")),
        ("modes", dirs("deliveries")),
        ("stores", stores),
    ];
    answer.words.push(("policy", pin.word.into()));
    answer.words.push(("identity", identity));
    answer.flagged = pin.word == "moved";
    Ok(answer)
}

/// One row per guard route (`.writfold/deliveries/guard.toml`).
fn route_rows(root: &Path) -> Vec<Row> {
    let Ok(Some(g)) = crate::deliver::config::guard(root) else {
        return Vec::new();
    };
    g.routes
        .iter()
        .map(|r| {
            let place = match &r.field {
                Some(f) => format!("field={f}"),
                None => format!("body={} file={}", r.body.join(","), r.file.join(",")),
            };
            Row {
                kind: "route",
                id: r.name(),
                text: format!("mode={} {place}", r.mode.as_deref().unwrap_or("any")),
                path: Some(g.path.clone()),
                line: Some(1),
                section: Some("policy"),
                ..Row::default()
            }
        })
        .collect()
}

/// Declared stores change what counts as evidence: each joins the
/// identity as one line (a KB without stores keeps the identity it had).
/// The number of stores.
fn store_lines(root: &Path, lines: &mut Vec<String>) -> Result<usize> {
    let stores = crate::store::read(root)?;
    for store in stores.iter() {
        let rule = match store.freshness {
            crate::store::Freshness::Pinned => "pinned",
            crate::store::Freshness::Head => "head",
        };
        let url = crate::store::normalise(&store.url);
        let recorded = (store.recorded_max_age.as_deref())
            .map(|age| format!("\trecorded={age}"))
            .unwrap_or_default();
        lines.push(format!("store:{}\t{rule}\t{url}{recorded}", store.name));
    }
    Ok(stores.iter().count())
}
