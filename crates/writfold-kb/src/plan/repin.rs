//! `plan repin --reason --by` (policy spec 2.5): a run whose policy moved
//! adopts the policy as it is now, on the record. One create-new pin of
//! kind `repin` ([`super::pin`]) through the write protocol, under the
//! plan re-check ([`super::recheck`]): every gate the plan names at its
//! definition digest now, a done step at the digest its done record names,
//! the required file's digest and lock list now, the reason and `by`.
//! Whether it may be written is [`writfold_core::policy::repin::repin_check`];
//! later reads compare with it because it is the newest pin naming each
//! gate ([`super::pin::gate_in_force`]). It invalidates nothing: evidence
//! under a changed definition was already stale. There is no multi-file
//! protocol to interrupt: before the file is published a rerun writes it,
//! after it a rerun is refused because nothing moved.
use super::drift::{self, Drift};
use super::pin::{self, Pin};
use super::pin_write::definition_now;
use super::recheck::{Decided, guard};
use super::{Loaded, PlanVerb, write};
use crate::Request;
use crate::emit::{More, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::Op;
use crate::{runs, time};
use std::path::Path;
use writfold_core::policy::repin::{Refusal, repin_check};

/// What a repin would pin for the plan's gates.
#[derive(Default)]
struct Adopted {
    gates: Vec<(String, String)>,
    steps: Vec<(String, String, String)>,
    /// The first gate that cannot be adopted, with why.
    missing: Option<String>,
}

/// Each gate the plan names, once, at its digest now; when only done
/// steps name it, at the digest the first of them records. A done step
/// whose record names another digest gets its own entry. A delivery is
/// pinned as it is now (it is not compared once done).
fn adopted(root: &Path, loaded: &Loaded) -> Adopted {
    let top = loaded.subject.as_ref().map(|s| s.top.as_path());
    let mut out = Adopted::default();
    let steps = &loaded.graph.steps;
    let words = super::agent_pins::with_agents(root, steps.iter().flat_map(|s| &s.gates));
    for gate in &words {
        if out.gates.iter().any(|(g, _)| g == gate) {
            continue;
        }
        let delivery =
            crate::deliver::step::is_delivery_word(gate) || super::agent_pins::is_agent_word(gate);
        let naming = || (steps.iter()).filter(|s| super::agent_pins::names(root, &s.gates, gate));
        // What each done step's record names for the gate.
        let mut done: Vec<(&str, Option<&str>)> = Vec::new();
        for (step, record) in naming().filter_map(|s| Some((s, &s.done.as_ref()?.1))) {
            let named =
                (record.met.iter().find(|m| &m.gate == gate)).map(|m| m.definition.as_str());
            done.push((&step.path, named));
        }
        let open = naming().any(|s| s.done.is_none());
        let owed = naming().find(|s| !s.closed());
        let general = if open || delivery || done.is_empty() {
            let now = definition_now(root, top, gate);
            if let (None, Some(step), None) = (&now, owed, &out.missing) {
                out.missing = Some(format!(
                    "{gate}, which step {} owes, does not load now; define it or restore its definition first",
                    step.path
                ));
            }
            now.unwrap_or_else(|| "undefined".into())
        } else {
            done.iter()
                .find_map(|(_, d)| *d)
                .unwrap_or("undefined")
                .to_owned()
        };
        for (step, named) in done.iter().filter(|_| !delivery) {
            match named {
                Some(d) if *d != general => {
                    out.steps
                        .push(((*step).to_owned(), gate.clone(), (*d).to_owned()));
                }
                Some(_) => {}
                None if out.missing.is_none() => {
                    out.missing = Some(format!(
                        "the done record of step {step} does not name {gate}; there is no definition to adopt for it"
                    ));
                }
                None => {}
            }
        }
        out.gates.push((gate.clone(), general));
    }
    out
}

/// The first step the run skipped that `locked`, the lock list as it is
/// now, would lock: its skip would then count for nothing and the step
/// would be open again.
fn reopened(root: &Path, loaded: &Loaded, locked: &[String]) -> Option<String> {
    let rec = &loaded.rec;
    let under = super::fold::read_under(root, &runs::dir(rec), &rec.id, locked);
    let skipped = |path: &str| {
        (loaded.graph.steps.iter())
            .any(|s| s.path == path && s.done.is_none() && s.skipped.is_some())
    };
    let step = under.steps.iter().find(|s| s.locked && skipped(&s.path))?;
    Some(step.path.clone())
}

/// Why a repin is refused (exit 3); nothing is written. `detail` names
/// the gate that does not load, or the step that would be open again.
fn refusal(why: &Refusal, loaded: &Loaded, drift: &Drift, detail: Option<String>) -> KbError {
    let run = &loaded.rec.id;
    let text = match why {
        Refusal::Closed => {
            return KbError::Refused(format!(
                "run {run} is closed; reopen it with status {run} --set open"
            ));
        }
        Refusal::Invalid => {
            let (path, why) = (drift.invalid.first())
                .map_or(("a pin file", "it does not read"), |(p, w)| {
                    (p.as_str(), w.as_str())
                });
            format!(
                "{path} no longer reads as written ({why}); restore it from Git first: a repin cannot adopt an edited pin"
            )
        }
        Refusal::Unpinned => format!(
            "run {run} has no policy pin (it has no plan, or its plan was set before pins were recorded), so there is nothing to adopt a change from"
        ),
        Refusal::Same => format!("nothing moved: run {run}'s policy is the one it pinned"),
        Refusal::Undefined => detail.unwrap_or_default(),
        Refusal::Reopens => format!(
            "the required steps file now makes step {} unskippable, and this run already skipped it; adopting that would open the step again: restore the file, or close this run and start another",
            detail.unwrap_or_default()
        ),
    };
    KbError::Refused(format!("not repinned: {text}; nothing written"))
}

/// `plan repin --reason --by`.
pub(super) fn repin(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    verb: &PlanVerb,
    now: i64,
) -> Result<Output> {
    let PlanVerb::Repin {
        reason,
        by,
        run,
        repo,
    } = verb
    else {
        return Err(KbError::Usage("plan repin".into()));
    };
    let reason = crate::gate::statement::check_text("--reason", reason)?;
    let by = crate::gate::statement::check_text("--by", by)?;
    let session = Session::open(ws, Some(tool), false, now)?;
    let loaded = Loaded::open(&session, run.as_deref(), repo.as_deref(), &request.cwd)?;
    let drift = drift::judge(&ws.root, &loaded);
    let adopted = adopted(&ws.root, &loaded);
    let facts = match drift.word {
        "moved" => writfold_core::policy::Pin::Moved,
        "same" => writfold_core::policy::Pin::Same,
        _ => writfold_core::policy::Pin::Unpinned,
    };
    let writable = write::writable(&loaded, &session);
    let required = super::pin_write::required(&ws.root, &loaded)?;
    let reopened = reopened(&ws.root, &loaded, &required.unskippable);
    let (valid, loads) = (drift.invalid.is_empty(), adopted.missing.is_none());
    if let Err(why) = repin_check(writable, valid, &facts, loads, reopened.is_none()) {
        let detail = adopted.missing.or(reopened);
        return Err(refusal(&why, &loaded, &drift, detail));
    }
    let run_dir = runs::dir(&loaded.rec);
    let seq = (drift.pins.iter().map(|p| p.seq).max().unwrap_or(0))
        .max(pin::max_seq(&ws.root, &run_dir))
        + 1;
    let new = Pin {
        path: String::new(),
        seq,
        kind: "repin".into(),
        covers: Vec::new(),
        required: Some(required),
        gates: adopted.gates,
        recorded: time::stamp(now),
        steps: adopted.steps,
        reason: Some(reason),
        by: Some(by),
    };
    let bytes = pin::bytes(&loaded.rec.id, &new);
    let path = pin::path(&run_dir, seq, &bytes);
    // The pin holds what the fold showed: the steps done must still be the
    // steps done when it is published.
    let current = loaded
        .graph
        .current()
        .and_then(|i| loaded.graph.steps.get(i));
    let decided = current.map_or(Decided::NoCurrent, |s| Decided::Current(s.path.clone()));
    let ops = [
        guard(&run_dir, &loaded.rec.id, decided),
        Op::Create {
            path: path.clone(),
            bytes,
        },
    ];
    let run_id = loaded.rec.id.clone();
    drop(session);
    write::apply(ws, request, &ops)?;
    let notes = vec![format!(
        "recorded {path} (the run adopts the policy as it is now; evidence under an earlier definition stays stale)"
    )];
    let target = ("plan-repin", run_id.as_str(), repo.as_deref());
    write::answer(ws, tool, request, target, notes, now)
}

/// What repin `at` of `pins` adopted: each gate with an entry that differs
/// from the one in force before it, and the required file when its digest
/// differs.
fn changed(pins: &[Pin], at: usize) -> Vec<String> {
    let (before, pin) = (&pins[..at], &pins[at]);
    let differs = |gate: &str, step: Option<&str>, digest: &str| {
        pin::gate_in_force(before, gate, step).is_none_or(|(d, _)| d != digest)
    };
    let mut out: Vec<String> = Vec::new();
    for (gate, digest) in &pin.gates {
        let stepped = (pin.steps.iter()).any(|(s, g, d)| g == gate && differs(g, Some(s), d));
        if differs(gate, None, digest) || stepped {
            out.push(gate.clone());
        }
    }
    let was = pin::required_in_force(before).map(|(_, r)| r.digest.as_str());
    if pin.required.as_ref().map(|r| r.digest.as_str()) != was {
        out.push("the required steps file".into());
    }
    out
}

/// One line per repin for `plan status`:
/// `repin: 1 (research, unit) by owner: <reason> @runs/…/policy/pin-3-….json:1`.
pub(super) fn lines(pins: &[Pin]) -> Vec<More> {
    let repins = (0..pins.len()).filter(|i| pins[*i].kind == "repin");
    repins
        .enumerate()
        .map(|(n, at)| {
            let pin = &pins[at];
            More {
                label: "repin",
                text: format!(
                    "{} ({}) by {}: {}",
                    n + 1,
                    changed(pins, at).join(", "),
                    pin.by.as_deref().unwrap_or("unknown"),
                    pin.reason.as_deref().unwrap_or("no reason recorded")
                ),
                path: Some(pin.path.clone()),
                line: Some(1),
                ..More::default()
            }
        })
        .collect()
}
