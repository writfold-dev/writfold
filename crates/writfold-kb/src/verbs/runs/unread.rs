//! Every file of a run, accounted for (#181, RFC
//! 2026-11-27-run-directory-accounting, law U): `resume` and `resume
//! --packet` list `runs/<id>/` and class each file as judged (a format a
//! Writfold reader takes), declared out (a dot-name) or left over
//! (another tool's files, a stray file). Left over is counted by the proved
//! [`writfold_core::account::remainder`], and a run with any reads
//! `coverage=partial`, never complete.
//!
//! Tested, not proved: the bounded walk and the classing table.
use crate::emit::{Answer, Row};
use std::path::Path;
use writfold_core::account::{Member, remainder};

/// The most entries the walk lists before it stops; a walk cut short is
/// itself left over.
pub const BOUND: usize = 20_000;

/// Directories whose reader takes its `.json` files.
const JSON_DIRS: [&str; 8] = [
    "gates",
    "statements",
    "covers",
    "observed",
    "plan",
    "policy",
    "artifacts",
    "invocations",
];

/// The directories of a delivery candidate whose reader takes `.json`.
const CANDIDATE_DIRS: [&str; 3] = ["checks", "statements", "posts"];

/// How one run-relative file is accounted for.
pub fn class(path: &str) -> Member {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.iter().any(|p| p.starts_with('.')) {
        return Member::DeclaredOut;
    }
    match parts.as_slice() {
        ["manifest.json" | "writfold-run.json" | "checkpoint.md"] => Member::Judged,
        [dir, file] if JSON_DIRS.contains(dir) && file.ends_with(".json") => Member::Judged,
        // A retained file of a gate's evidence: any name, bound by its receipt.
        ["evidence", _, _] => Member::Judged,
        // The plan template a splice pinned, by digest.
        ["plan", "templates", file] if file.ends_with(".toml") => Member::Judged,
        ["deliveries", _, "delivery.json" | "candidate.md"] => Member::Judged,
        ["deliveries", _, dir, file] if CANDIDATE_DIRS.contains(dir) && file.ends_with(".json") => {
            Member::Judged
        }
        _ => Member::Remainder,
    }
}

/// The run's files as members, with the left-over paths and why the walk
/// was not whole (a directory it could not list, the bound).
pub struct Account {
    pub members: Vec<Member>,
    pub left: Vec<String>,
}

fn walk(dir: &Path, rel: &str, out: &mut Account) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        out.members.push(Member::Remainder);
        out.left.push(format!("{rel}/ (could not be listed)"));
        return;
    };
    for entry in entries.flatten() {
        if out.members.len() >= BOUND {
            out.members.push(Member::Remainder);
            out.left
                .push(format!("(more than {BOUND} entries; the listing stopped)"));
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        // A symbolic link is listed, never followed.
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir && !name.starts_with('.') {
            walk(&entry.path(), &path, out);
            continue;
        }
        let member = class(&path);
        if member == Member::Remainder {
            out.left.push(path);
        }
        out.members.push(member);
    }
}

/// Account for every file under `root/run_dir`.
pub fn account(root: &Path, run_dir: &str) -> Account {
    let mut out = Account {
        members: Vec::new(),
        left: Vec::new(),
    };
    walk(&root.join(run_dir), "", &mut out);
    out
}

/// `observations/ (3), strange.bin`: the left-over paths by their first
/// directory, at most five names, then `(+N more)`.
fn tally(left: &[String]) -> String {
    let mut groups: Vec<(String, usize)> = Vec::new();
    for path in left {
        // A directory that could not be listed, or the bound, is named as is.
        let key = match path.split_once('/') {
            _ if path.contains(" (") => path.clone(),
            Some((dir, _)) => format!("{dir}/"),
            None => path.clone(),
        };
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, n)) => *n += 1,
            None => groups.push((key, 1)),
        }
    }
    groups.sort();
    let shown: Vec<String> = groups
        .iter()
        .take(5)
        .map(|(k, n)| {
            if k.ends_with('/') {
                format!("{k} ({n})")
            } else {
                k.clone()
            }
        })
        .collect();
    let more = groups.len().saturating_sub(5);
    match more {
        0 => shown.join(", "),
        n => format!("{} (+{n} more)", shown.join(", ")),
    }
}

/// Add the `unread` row, count and note to a `resume` answer; nothing when
/// every file was accounted for, so such a run reads as before.
pub fn add(answer: &mut Answer, root: &Path, run_dir: &str) {
    let found = account(root, run_dir);
    let unread = remainder(&found.members);
    if unread == 0 {
        return;
    }
    let names = tally(&found.left);
    answer.rows.push(Row {
        kind: "unread",
        id: format!("{run_dir}/"),
        status: Some("unread".into()),
        text: format!("{unread} files in {run_dir}/ are in no format Writfold reads: {names}"),
        path: Some(format!("{run_dir}/")),
        flags: vec!["unread".into()],
        ..Row::default()
    });
    answer.counts.push(("unread", unread));
    answer.partial = true;
    answer.notes.push(format!(
        "{unread} file(s) of the run were not read ({names}); they are not evidence here and are not in a packet"
    ));
}

#[cfg(test)]
mod tests {
    use super::{Member, class, tally};

    #[test]
    fn files_are_classed_by_the_table() {
        for judged in [
            "manifest.json",
            "checkpoint.md",
            "gates/unit-20261101T000000Z-abcdef.json",
            "plan/done-step.json",
            "policy/pin-1-ab.json",
            "plan/templates/9167456b.toml",
            "evidence/unit-x/report.txt",
            "deliveries/c1/candidate.md",
            "deliveries/c1/checks/x.json",
            "invocations/i1.json",
        ] {
            assert_eq!(class(judged), Member::Judged, "{judged}");
        }
        for out in [".gitkeep", "gates/.tmp-x", ".cache/a"] {
            assert_eq!(class(out), Member::DeclaredOut, "{out}");
        }
        for left in [
            "observations/o1.yaml",
            "receipts/r.json",
            "strange.bin",
            "gates/unit.txt",
            "gates/nested/unit.json",
            "deliveries/stray.txt",
            "deliveries/c1/foreign.bin",
            "invocations/i1/package/input.md",
        ] {
            assert_eq!(class(left), Member::Remainder, "{left}");
        }
    }

    #[test]
    fn the_tally_names_at_most_five() {
        let left: Vec<String> = ["observations/a", "observations/b", "x.bin"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(tally(&left), "observations/ (2), x.bin");
        let many: Vec<String> = (0..7).map(|i| format!("d{i}/f")).collect();
        assert!(tally(&many).ends_with("(+2 more)"));
    }
}
