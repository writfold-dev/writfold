//! `prose.vale` (#117, RFC 2026-11-25-vale-for-documents): run the user's
//! `vale` on the payload with the mode's pinned, bound profile.
//!
//! The payload is copied to `document.md` in a fresh private directory and
//! the profile's canary to `canary.md` beside it. `vale --version` is
//! probed first; then one run, `vale --no-global --config <tree>/.vale.ini
//! --output=JSON --counts document.md canary.md`, in that directory, with
//! every `VALE_*` variable removed and `HOME` and `XDG_CONFIG_HOME` set to
//! it. `WRITFOLD_VALE` names another executable. Writfold never installs
//! Vale and never runs `vale sync`.
//!
//! The decision is [`writfold_core::vale::settle`]: anything but a whole
//! report from the pinned version, whose exit status agrees with its
//! alerts, is `unknown`, and so is a run that applied no rule. Running,
//! isolation, parsing and rendering are tested, not proved.
use super::checks::{Evaluated, Finding};
use super::vale_profile::{Vale, block_name};
use super::vale_report::{self as report, Alert, CANARY, DOCUMENT, level_name};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use writfold_core::gate::{Ending, Outcome};
use writfold_core::prose::Weight;
use writfold_core::vale::{Level, Ran, Tool, settle, weigh};
use writfold_runtime::process::{Tail, TailLimits, TailedEnd, run_tailed};

/// The largest report read.
pub const MAX_OUTPUT: usize = 8 << 20;
/// The longest finding text kept, in characters.
const MAX_DETAIL: usize = 200;

static PROGRAM: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Use `path` as the Vale executable for the rest of this process (tests
/// and embedders, which cannot set `WRITFOLD_VALE`); the first call wins.
pub fn use_program(path: &str) {
    // A second call keeps the first program, as documented: its error is
    // the value it would have set.
    if PROGRAM.set(path.to_owned()).is_err() {
        return;
    }
}

/// The executable: [`use_program`]'s, else `WRITFOLD_VALE`, else `vale` on
/// `PATH`.
pub fn program() -> String {
    match PROGRAM.get() {
        Some(p) => p.clone(),
        None => std::env::var("WRITFOLD_VALE").unwrap_or_else(|_| "vale".into()),
    }
}

/// The command that lists every alert natively, for the note.
pub fn native(root: &Path, v: &Vale) -> String {
    let config = root.join(v.profile.tree()).join(super::vale_tree::CONFIG);
    format!("vale --no-global --config {} <payload>", config.display())
}

struct Run {
    ending: Ending,
    stdout: Tail,
    stderr: Tail,
    error: Option<String>,
}

fn run(dir: &Path, args: &[&str], tail: usize, timeout: u64) -> Run {
    let mut command = Command::new(program());
    command.args(args).current_dir(dir);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("VALE_") {
            command.env_remove(key);
        }
    }
    command.env("HOME", dir).env("XDG_CONFIG_HOME", dir);
    let limits = TailLimits {
        tail,
        timeout: Duration::from_secs(timeout),
        grace: Duration::from_secs(5),
    };
    let failed = |why: String| Run {
        ending: Ending::Error,
        stdout: Tail::default(),
        stderr: Tail::default(),
        error: Some(why),
    };
    let supervisor = match crate::workspace::supervisor() {
        Ok(s) => s,
        Err(e) => return failed(e.to_string()),
    };
    match run_tailed(&mut command, limits, supervisor) {
        Ok(done) => Run {
            ending: match done.end {
                TailedEnd::Exited(code) => Ending::Exited(code),
                TailedEnd::Signaled(_) => Ending::Signaled,
                TailedEnd::TimedOut => Ending::TimedOut,
            },
            stdout: done.stdout,
            stderr: done.stderr,
            error: None,
        },
        Err(e) => failed(format!("could not run {}: {e}", program())),
    }
}

/// A fresh private directory holding the two files.
fn stage(payload: &str, canary: &str) -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!("writfold-vale-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&dir)?;
    std::fs::write(dir.join(DOCUMENT), payload)?;
    std::fs::write(dir.join(CANARY), format!("{canary}\n"))?;
    Ok(dir)
}

fn unknown(note: String) -> Evaluated {
    Evaluated {
        outcome: Outcome::Unknown,
        findings: Vec::new(),
        advice: Vec::new(),
        note: Some(note),
    }
}

/// Evaluate a payload with the mode's Vale profile.
pub fn evaluate(root: &Path, payload: &str, v: &Vale) -> Evaluated {
    let p = &v.profile;
    let dir = match stage(payload, &p.canary) {
        Ok(dir) => dir,
        Err(e) => return unknown(format!("could not stage the document for Vale: {e}")),
    };
    let evaluated = evaluate_in(root, &dir, v);
    // The private directory is under the temporary directory; failing to
    // remove it does not change what Vale reported.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "a leftover temporary directory does not change the verdict"
    )]
    let _ = std::fs::remove_dir_all(&dir);
    evaluated
}

fn evaluate_in(root: &Path, dir: &Path, v: &Vale) -> Evaluated {
    let p = &v.profile;
    let probe = run(dir, &["--version"], 4096, 10);
    let found =
        matches!(probe.ending, Ending::Exited(0)).then(|| report::version(&probe.stdout.bytes));
    let tool = match found.flatten() {
        None => {
            let why = probe
                .error
                .unwrap_or_else(|| "it printed no version".into());
            return unknown(format!(
                "{} did not run ({why}); to fix: install vale {} (pinned in {})",
                program(),
                p.version,
                p.path
            ));
        }
        Some(got) if got != p.version => {
            return unknown(format!(
                "vale {got} is installed, but {} pins {}; to fix: install vale {} or move the pin",
                p.path, p.version, p.version
            ));
        }
        Some(_) => Tool::Pinned,
    };
    let config = root.join(p.tree()).join(super::vale_tree::CONFIG);
    let config = config.to_string_lossy();
    let args = [
        "--no-global",
        "--config",
        &config,
        "--output=JSON",
        "--counts",
        DOCUMENT,
        CANARY,
    ];
    let r = run(dir, &args, MAX_OUTPUT, p.timeout_seconds);
    let cut = r.stdout.total > r.stdout.bytes.len() as u64;
    let read = (!cut)
        .then(|| report::read(&r.stdout.bytes, &dir.to_string_lossy()))
        .flatten();
    let rep = read.clone().unwrap_or_default();
    let ran = Ran {
        tool,
        ending: r.ending,
        whole: read.is_some(),
        errors: rep.errors,
        enabled: rep.enabled,
        canary: rep.canary,
    };
    let weights: Vec<Weight> = rep
        .document
        .iter()
        .map(|a| weigh(a.level, v.block))
        .collect();
    let outcome = settle(ran, &weights);
    if outcome == Outcome::Unknown {
        return unknown(why_unknown(&r, cut, read.as_ref(), p.timeout_seconds));
    }
    recorded(outcome, &rep.document, &weights, (root, v, rep.enabled))
}

/// Why a run from the pinned version is `unknown`, with what to do.
fn why_unknown(r: &Run, cut: bool, read: Option<&report::Report>, timeout: u64) -> String {
    match (r.ending, read) {
        (Ending::Exited(code), _) if !(0..2).contains(&code) => format!(
            "vale exited {code}: {}; to fix: run the native command and correct the profile",
            report::error(&r.stderr.bytes)
        ),
        (Ending::Signaled, _) => "vale was stopped by a signal".into(),
        (Ending::TimedOut, _) => format!(
            "vale ran past its timeout of {timeout} s; to fix: raise timeout_seconds in the profile"
        ),
        (Ending::Error, _) => format!(
            "vale did not run: {}",
            r.error.clone().unwrap_or_else(|| "unknown error".into())
        ),
        (_, None) if cut => format!("vale printed more than {MAX_OUTPUT} bytes"),
        (_, None) => "vale's report is not the --output=JSON --counts shape (unknown severity, stray path or malformed JSON)".into(),
        (_, Some(rep)) if rep.enabled == 0 => {
            "vale loaded no check; to fix: name your styles in BasedOnStyles in .vale.ini".into()
        }
        (_, Some(rep)) if rep.canary == 0 => "the canary drew no alert, so the config may match nothing; to fix: check the [*.md] section of .vale.ini or the canary line".into(),
        (_, Some(_)) => "vale's exit status disagrees with its alerts".into(),
    }
}

fn rank(level: Level) -> u8 {
    match level {
        Level::Error => 0,
        Level::Warning => 1,
        Level::Suggestion => 2,
    }
}

fn finding(a: &Alert) -> Finding {
    let detail = format!(
        "column {}: {}: {}",
        a.column,
        level_name(a.level),
        a.message
    );
    Finding {
        line: a.line,
        rule: a.check.clone(),
        detail: detail.chars().take(MAX_DETAIL).collect(),
    }
}

/// The receipt's findings (blocking, by line), advice (by severity, then
/// line) and note (version, checks, totals, the five most frequent rules).
fn recorded(
    outcome: Outcome,
    alerts: &[Alert],
    weights: &[Weight],
    (root, v, enabled): (&Path, &Vale, usize),
) -> Evaluated {
    let mut blocking: Vec<&Alert> = Vec::new();
    let mut advice: Vec<&Alert> = Vec::new();
    for (a, w) in alerts.iter().zip(weights) {
        match w {
            Weight::Blocking => blocking.push(a),
            Weight::Advice => advice.push(a),
        }
    }
    blocking.sort_by_key(|a| (a.line, a.column));
    advice.sort_by_key(|a| (rank(a.level), a.line, a.column));
    let by_level = |l: Level| alerts.iter().filter(|a| a.level == l).count();
    let mut tally: Vec<(&str, usize)> = Vec::new();
    for a in alerts {
        match tally.iter_mut().find(|(c, _)| *c == a.check) {
            Some((_, n)) => *n += 1,
            None => tally.push((&a.check, 1)),
        }
    }
    tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let most: Vec<String> = tally
        .iter()
        .take(5)
        .map(|(c, n)| format!("{c} {n}"))
        .collect();
    let mut note = format!(
        "vale {}; {enabled} checks; blocks from {}; {} alerts: {} error, {} warning, {} suggestion",
        v.profile.version,
        block_name(v.block),
        alerts.len(),
        by_level(Level::Error),
        by_level(Level::Warning),
        by_level(Level::Suggestion),
    );
    if !most.is_empty() {
        note.push_str(&format!("; most: {}", most.join(", ")));
    }
    note.push_str(&format!("; all alerts: {}", native(root, v)));
    Evaluated {
        outcome,
        findings: blocking.into_iter().map(finding).collect(),
        advice: advice.into_iter().map(finding).collect(),
        note: Some(note),
    }
}
