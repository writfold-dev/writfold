//! One event at the guard site (#183, #184): the delivery guard, then the
//! rules, each judged where the command it matched acts
//! ([`super::matched`]), then the posture. `kb hook pre-tool-use` renders
//! the result as Claude Code expects; `kb guard check` prints it, logs
//! `via: check` and exits by it.
//!
//! An event the site cannot fully decide is undecided: the site file does
//! not parse, a rule file or directory cannot be read, or a required rule
//! did not load ([`causes`]). Its decision is the judged one joined with
//! the posture's ([`writfold_core::guard::combine`]): under `open` the
//! posture adds a pass, so the decision is today's; under `ask` or `deny`
//! it never passes, and the reason names the cause and the way out.
use super::posture::{self, FILE};
use super::{delivery, matched, pre, problems_note};
use crate::workspace::Workspace;
use serde_json::{Map, Value};
use std::path::Path;
use writfold_core::guard::{Decision, combine};
use writfold_core::site::{self as core, Basis, Posture};

/// What the site decided for one event.
pub struct Sited {
    pub decision: Decision,
    pub basis: Basis,
    pub posture: Posture,
    pub reasons: Vec<String>,
    pub stderr: String,
    /// The delivery guard denied it (its reason is the only one).
    pub delivery: bool,
    /// One row per judged rule.
    pub rules: Vec<Value>,
    /// Why the event is undecided, when it is.
    pub causes: Vec<String>,
}

impl Sited {
    pub fn passed(basis: Basis, posture: Posture, stderr: String) -> Self {
        Sited {
            decision: Decision::Pass,
            basis,
            posture,
            reasons: Vec::new(),
            stderr,
            delivery: false,
            rules: Vec::new(),
            causes: Vec::new(),
        }
    }

    /// An event the site could not decide at all (no KB, input that is not
    /// an event): the posture's decision.
    pub fn undecided(posture: Posture, cause: String, stderr: String) -> Self {
        let mut out = Sited::passed(Basis::Undecided, posture, stderr);
        out.causes.push(cause);
        out.settle();
        out
    }

    /// Join the posture's decision for the causes into the judged one.
    fn settle(&mut self) {
        if self.causes.is_empty() {
            self.decision = core::decide(self.basis, self.posture);
            return;
        }
        let undecided = core::decide(Basis::Undecided, self.posture);
        self.decision = combine(self.decision, undecided);
        if undecided != Decision::Pass {
            self.basis = Basis::Undecided;
            self.reasons.push(reason(self.posture, &self.causes));
        }
    }
}

/// The reason an undecided event did not pass.
pub fn reason(posture: Posture, causes: &[String]) -> String {
    format!(
        "writfold guard (posture {}; a guard, not enforcement): could not decide: {}; \
         fix it (writfold kb lint), or a person sets posture = \"open\" in {FILE}",
        posture.name(),
        causes.join("; ")
    )
}

/// Why the loaded rules leave the event undecided: a file or directory
/// that did not read or parse (an ID defined twice is not one), and a
/// required rule that did not load.
fn causes(loaded: Option<&matched::Loaded>, required: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for problem in loaded.into_iter().flat_map(|l| &l.problems) {
        if problem
            .report
            .diagnostics
            .iter()
            .all(|d| d.code != "file.twice")
        {
            out.push(problem.why());
        }
    }
    for id in required {
        if !loaded.is_some_and(|l| l.has(id)) {
            out.push(format!("required rule {id} did not load"));
        }
    }
    out
}

/// The effective posture and the site file's required rules; an
/// unparsable file is a cause.
fn site_file(ws: &Workspace, registration: Posture) -> (Posture, Vec<String>, Option<String>) {
    match posture::read(&ws.root) {
        Ok(Some(site)) => (
            core::stricter(registration, site.posture),
            site.required,
            None,
        ),
        Ok(None) => (registration, Vec::new(), None),
        Err(why) => (registration, Vec::new(), Some(why)),
    }
}

/// Keep the unclassified commands a `log` policy passed (#187); a log
/// that cannot be written changes no decision.
fn log_unclassified(
    ws: &Workspace,
    input: &Map<String, Value>,
    commands: &[String],
    now: i64,
) -> String {
    let text = |k: &str| input.get(k).and_then(Value::as_str).unwrap_or("");
    let entries = super::outbound::entries(text("session_id"), text("tool_name"), commands, now);
    match (super::log::path(ws), entries.is_empty()) {
        (Some(path), false) => match super::log::append(&path, &entries) {
            Ok(()) => String::new(),
            Err(why) => format!("writfold kb hook: {why}; the decision is unchanged\n"),
        },
        _ => String::new(),
    }
}

/// Decide one event, given as the canonical map ([`super::event`]), under
/// the registration's posture.
pub fn decide(
    ws: &Workspace,
    input: &Map<String, Value>,
    (cwd, home): (&Path, Option<&Path>),
    now: i64,
    registration: Posture,
) -> Sited {
    let (posture, required, broken) = site_file(ws, registration);
    let mut stderr = String::new();
    let mut asked = Vec::new();
    match delivery::judge(ws, input, cwd, now) {
        Ok(Some(v)) if v.decision == Decision::Deny => {
            let mut out = Sited::passed(Basis::Judged(Decision::Deny), posture, stderr);
            out.decision = Decision::Deny;
            out.reasons = v.reasons;
            out.delivery = true;
            return out;
        }
        Ok(Some(v)) => {
            stderr.push_str(&log_unclassified(ws, input, &v.logged, now));
            if v.decision == Decision::Ask {
                asked = v.reasons;
            }
        }
        Ok(None) => {}
        Err(why) => stderr = format!("writfold kb hook: {why}\n"),
    }
    let call = matched::Call::read(input, cwd, home);
    let loaded = matched::Loaded::find(&ws.root, &call.places());
    let mut out = match &loaded {
        None => Sited::passed(Basis::NoRules, posture, stderr),
        Some(loaded) => judged(ws, input, (loaded, &call), now, (posture, stderr)),
    };
    if !asked.is_empty() {
        out.decision = combine(out.decision, Decision::Ask);
        out.basis = Basis::Judged(out.decision);
        asked.append(&mut out.reasons);
        out.reasons = asked;
    }
    out.causes = broken
        .into_iter()
        .chain(causes(loaded.as_ref(), &required))
        .collect();
    out.settle();
    out
}

fn judged(
    ws: &Workspace,
    input: &Map<String, Value>,
    (loaded, call): (&matched::Loaded, &matched::Call<'_>),
    now: i64,
    (posture, mut stderr): (Posture, String),
) -> Sited {
    stderr.push_str(&problems_note(&loaded.problems));
    let groups = matched::groups(loaded, call);
    if groups.is_empty() {
        return Sited::passed(Basis::NoMatch, posture, stderr);
    }
    let decided = pre::judge(ws, input, &groups, now);
    stderr.push_str(&decided.stderr);
    let mut out = Sited::passed(Basis::Judged(decided.decision), posture, stderr);
    out.decision = decided.decision;
    out.reasons = decided.reasons;
    out.rules = decided.rules;
    out
}
