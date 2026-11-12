//! A mode's script gate, run against a candidate's payload file by
//! `deliver check` (and by nothing else). The command is the gate's argv
//! with `{candidate}` set to the payload file and `{gate}` to the gate's
//! directory, run in its working directory under the KB root with G1's
//! supervision (process group, timeout, bounded output).
//!
//! The script reports findings on standard output, one per line, as
//! `<line>: <text>` (or `line <line>: <text>`); any other non-empty line is
//! a finding about the whole candidate (line 0). Exit 0 is a pass and its
//! output is ignored; any other exit status is a fail, with at least one
//! finding; a signal, a timeout, a failure to start, or the candidate
//! changing while the script ran is `unknown`, never a pass.
use super::candidate::{Candidate, sha};
use super::checks::{Evaluated, Finding};
use crate::gate::def::Definition;
use crate::gate::exec;
use std::path::Path;
use writfold_core::gate::{Ending, Outcome};

/// The longest finding text kept, in characters.
const MAX_DETAIL: usize = 200;

/// One stdout line as a finding.
fn finding(gate: &str, text: &str) -> Finding {
    let rest = text.strip_prefix("line ").unwrap_or(text);
    let numbered = rest
        .split_once(':')
        .and_then(|(n, d)| n.trim().parse::<u32>().ok().map(|n| (n, d.trim())))
        .filter(|(_, d)| !d.is_empty());
    let (line, detail) = numbered.unwrap_or((0, text.trim()));
    Finding {
        line,
        rule: gate.to_owned(),
        detail: detail.chars().take(MAX_DETAIL).collect(),
    }
}

/// The findings a script printed.
pub fn findings(gate: &str, stdout: &[u8]) -> Vec<Finding> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| finding(gate, l))
        .collect()
}

/// Run gate `d` against candidate `c` (whose payload is under `root`).
pub fn evaluate(root: &Path, c: &Candidate, d: &Definition) -> Evaluated {
    let before = c.digest();
    let payload = root.join(c.payload_path());
    let e = exec::run_on_candidate(d, root, &payload);
    let after = std::fs::read(&payload).map_or_else(|_| "missing".into(), |b| sha(&b));
    let unknown = |note: String| Evaluated {
        outcome: Outcome::Unknown,
        findings: Vec::new(),
        note: Some(note),
    };
    if after != before {
        return unknown("the candidate changed while the gate ran".into());
    }
    match e.ending {
        Ending::Exited(0) => Evaluated {
            outcome: Outcome::Pass,
            findings: Vec::new(),
            note: Some("exit=0".into()),
        },
        Ending::Exited(code) => {
            let mut found = findings(&d.id, &e.stdout.bytes);
            if found.is_empty() {
                found.push(Finding {
                    line: 0,
                    rule: d.id.clone(),
                    detail: format!("exit={code} with no finding printed"),
                });
            }
            Evaluated {
                outcome: Outcome::Fail,
                findings: found,
                note: Some(format!("exit={code}")),
            }
        }
        Ending::Signaled => unknown(format!(
            "stopped by signal {}",
            e.signal.unwrap_or_default()
        )),
        Ending::TimedOut => unknown(format!(
            "ran past its timeout of {} s and was stopped",
            d.timeout_seconds
        )),
        Ending::Error => unknown(format!(
            "did not run: {}",
            e.error.unwrap_or_else(|| "unknown error".into())
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::findings;

    #[test]
    fn stdout_lines_become_findings() {
        let found = findings(
            "commit-format",
            b"1: subject has 81 characters\n\nline 4: body line has 90 characters\nno line here\n7:\n",
        );
        let shown: Vec<(u32, &str)> = found.iter().map(|f| (f.line, f.detail.as_str())).collect();
        assert_eq!(
            shown,
            [
                (1, "subject has 81 characters"),
                (4, "body line has 90 characters"),
                (0, "no line here"),
                (0, "7:"),
            ]
        );
        assert!(found.iter().all(|f| f.rule == "commit-format"));
    }
}
