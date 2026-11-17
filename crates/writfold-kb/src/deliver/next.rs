//! `next:` under a candidate, from its first obligation not met under its
//! policy: re-check when the receipt is missing or about other bytes; edit
//! the named lines, then check, when a check failed; ask for the deny list
//! when there is none (it is the team's policy); attest last; and no
//! command for a statement the policy does not take (`why:` says why).
use super::candidate::Candidate;
use super::config::REFERENCES;
use super::status::Judged;
use std::path::Path;
use writfold_core::gate::{Standing, State};

/// `line 5: internal-host, ticket; line 7: paragraphs.bounded`, from the
/// receipt's first findings.
fn lines(j: &Judged<'_>) -> String {
    let Some((_, r)) = j.receipt else {
        return String::new();
    };
    let mut by_line: Vec<(u32, Vec<&str>)> = Vec::new();
    for f in &r.findings {
        match by_line.iter_mut().find(|(n, _)| *n == f.line) {
            Some((_, rules)) if !rules.contains(&f.rule.as_str()) => rules.push(&f.rule),
            Some(_) => {}
            None => by_line.push((f.line, vec![&f.rule])),
        }
    }
    let shown: Vec<String> = by_line
        .iter()
        .take(3)
        .map(|(n, rules)| format!("line {n}: {}", rules.join(", ")))
        .collect();
    match shown.is_empty() {
        true => String::new(),
        false => format!(" ({})", shown.join("; ")),
    }
}

/// The command (or instruction) that meets the first unmet obligation of
/// `c`; `None` when every obligation is met, or the first unmet one is a
/// statement its policy does not take.
pub fn next(root: &Path, c: &Candidate, judged: &[Judged<'_>]) -> Option<String> {
    let _ = root;
    let j = judged.iter().find(|j| j.unmet())?;
    let id = &c.id;
    let check = format!("writfold kb deliver check {id}");
    if j.rejected() {
        return None;
    }
    if j.obligation.class == "attested" {
        return Some(format!(
            "writfold kb deliver attest {id} --check {} --statement TEXT --by WHO",
            j.obligation.name
        ));
    }
    let no_list = j.obligation.name == REFERENCES
        && j.standing == Standing::Checked(State::Unknown)
        && j.receipt.is_some_and(|(_, r)| {
            r.note
                .as_deref()
                .is_some_and(|n| n.contains("no deny list"))
        });
    if no_list {
        let audience = c
            .manifest
            .as_ref()
            .map_or("internal", |m| m.audience.as_str());
        return Some(format!(
            "ask for {}/deny-{audience}.toml (the team's policy), then: {check}",
            super::config::DIR
        ));
    }
    match j.standing {
        Standing::Checked(State::Failed) => Some(format!(
            "edit {}{}, then: {check}",
            c.payload_path(),
            lines(j)
        )),
        _ => Some(check),
    }
}

/// D5's line for a ready candidate with no post record about its present
/// bytes: where to post the payload file, then how to record it.
pub fn post(root: &Path, c: &Candidate) -> String {
    let to = c
        .manifest
        .as_ref()
        .ok()
        .and_then(|m| m.destination.clone())
        .unwrap_or_else(|| "its destination".into());
    format!(
        "post {} to {to}, then: writfold kb deliver posted {} --ref URL --by WHO",
        super::found::Found::path_of(root, c),
        c.id
    )
}
