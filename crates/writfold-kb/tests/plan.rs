//! The run graph, linear: `plan set` splices a template at the run's root,
//! `plan advance` refuses (exit 3, nothing written) until every gate of
//! the current step is met and then records what met each, `plan status`
//! derives every step from the event files, a done step whose gate breaks
//! shows `!stale` without moving the current step, `gate status` and
//! `resume` narrow to the plan's gates, and run-record checks judge the
//! run's own records. The fold depends only on the set of event files.
mod support;

use std::path::Path;
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::Write;
use writfold_kb::write::record::Ticket;
use writfold_kb::write::run::Checkpoint;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"grep\", \"-q\", \"append\", \"src/ledger.rs\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
const LINT: &str = "schema = \"writfold-gate/1\"\nid = \"lint\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n";
const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";
const BACKFILLED: &str = "schema = \"writfold-gate/1\"\nid = \"backfilled\"\nversion = 1\nclass = \"run-record\"\nrequire = [\"research.backfilled\"]\n";
const REFLECTED: &str = "schema = \"writfold-gate/1\"\nid = \"reflected\"\nversion = 1\nclass = \"run-record\"\nrequire = [\"run.reflected\"]\n";
pub const CHANGE: &str = "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\n  \"research: backfilled\",\n  \"implement: unit\",\n  \"review: review\",\n  \"reflect: reflected\",\n]\n";

fn stable(text: &str) -> String {
    let text = text.replace(RUN, "<RUN>");
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}").ok();
    let text = names.map_or(text.clone(), |re| {
        re.replace_all(&text, "-$1-XXXXXX").into_owned()
    });
    text.replace("<RUN>", RUN)
}

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{}# exit={exit}\n", stable(text));
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// The KB's gates and the `change` template.
pub fn define(w: &World) -> TestResult {
    for (id, text) in [
        ("unit", UNIT),
        ("lint", LINT),
        ("review", REVIEW),
        ("backfilled", BACKFILLED),
        ("reflected", REFLECTED),
    ] {
        put(&w.kb.join(format!(".writfold/gates/{id}.toml")), text)?;
    }
    put(&w.kb.join(".writfold/plans/change.toml"), CHANGE)
}

fn call(w: &World, verb: Verb) -> TestResult<(String, u8)> {
    w.run(&w.request(verb))
}

fn plan(w: &World, verb: &str) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let verb = match verb {
        "set" => PlanVerb::Set {
            template: Some("change".into()),
            from: None,
            run,
            repo,
        },
        "status" => PlanVerb::Status { run, repo },
        _ => PlanVerb::Advance { run, repo },
    };
    call(w, Verb::Plan(verb))
}

fn gate(w: &World, id: &str) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}

fn note(w: &World, text: &str) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Write(Write::Checkpoint(Checkpoint {
            run: Some(RUN.into()),
            new_run: None,
            note: text.into(),
        })),
    )
}

fn events(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Advance is refused, then allowed, step by step; status, resume and
/// `gate status` show the plan; a broken gate marks its done step stale.
#[test]
fn a_plan_is_walked_step_by_step() -> TestResult {
    let owned = world(&[])?;
    let w = &owned;
    define(w)?;
    let gates = Verb::Gate(GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let (text, _) = call(w, gates.clone())?;
    assert!(
        text.contains("gate lint ["),
        "no plan: every gate is owed\n{text}"
    );
    let (text, exit) = plan(w, "status")?;
    assert!(exit == 2 && text.contains("has no plan"), "{text}");
    let (text, exit) = plan(w, "set")?;
    golden("plan-set.txt", &text, exit)?;
    let (again, exit) = plan(w, "set")?;
    assert!(exit == 0 && again.contains("already set"), "{again}");
    assert_eq!(events(w).len(), 2, "{:?}", events(w));
    // `gate status` now owes only the plan's gates.
    let (text, _) = call(w, gates.clone())?;
    assert!(
        !text.contains("gate lint [") && text.contains("gate unit ["),
        "{text}"
    );
    // research: nothing written yet, so the run-record check fails.
    let (text, exit) = plan(w, "advance")?;
    golden("plan-advance-refused.txt", &text, exit)?;
    assert_eq!(events(w).len(), 2, "a refused advance writes nothing");
    let (text, exit) = gate(w, "backfilled")?;
    assert!(exit == 2 && text.contains("no KB record changed"), "{text}");
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-300".into(),
        title: "Month-end replay is slow".into(),
        ..Ticket::default()
    });
    let (text, exit) = call(w, Verb::Write(ticket))?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = gate(w, "backfilled")?;
    assert!(exit == 0 && text.contains("[satisfied]"), "{text}");
    let (text, exit) = plan(w, "advance")?;
    assert!(
        exit == 0 && text.contains("step change#1/research [done]"),
        "{text}"
    );
    // implement: the unit gate, open then satisfied.
    let (text, exit) = plan(w, "advance")?;
    assert!(exit == 3 && text.contains("owes: unit [open]"), "{text}");
    gate(w, "unit")?;
    let (text, exit) = plan(w, "advance")?;
    assert_eq!(exit, 0, "{text}");
    stale_and_resume(w)?;
    finish(w)
}

/// Review is current; breaking unit marks implement `!stale`, and the
/// current step stays where it was.
fn stale_and_resume(w: &World) -> TestResult {
    put(&w.code.join("src/ledger.rs"), "pub fn write() {}\n")?;
    let (text, exit) = plan(w, "status")?;
    assert!(
        exit == 2 && text.contains("step change#1/implement [done] unit [stale]"),
        "{text}"
    );
    assert!(
        text.contains("!stale") && text.contains("step change#1/review [current]"),
        "{text}"
    );
    assert!(
        text.contains("step change#1/research [done] backfilled [satisfied]"),
        "{text}"
    );
    // From inside the subject repository, as an agent would ask.
    let mut resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    resume.cwd = w.code.clone();
    let (text, _) = w.run(&resume)?;
    assert!(
        text.contains("plan change#1/review [current] step 3/4 owes: review [open]"),
        "{text}"
    );
    git(&w.code, &["checkout", "--", "src/ledger.rs"])?;
    Ok(())
}

/// Review: an attestation; reflect: a reflection and "no proposal"; then
/// the plan is complete and `kb check` shows its part.
fn finish(w: &World) -> TestResult {
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "review".into(),
        text: "read the diff".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    call(w, Verb::Gate(say))?;
    assert_eq!(plan(w, "advance")?.1, 0);
    let (text, exit) = gate(w, "reflected")?;
    assert!(exit == 2 && text.contains("reflection:"), "{text}");
    note(
        w,
        "Reflection: the plan's steps were the right size.\nNo proposal: nothing to change.",
    )?;
    let (text, exit) = gate(w, "reflected")?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = plan(w, "advance")?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = plan(w, "status")?;
    golden("plan-status-complete.txt", &text, exit)?;
    let complete = plan(w, "advance");
    assert!(complete.is_err_and(|e| e.to_string().contains("no current step")));
    let (text, _) = call(w, Verb::Check { scope: None })?;
    assert!(text.contains("check plan [clean]"), "{text}");
    Ok(())
}

/// A receipt's judging order is its `seq`, never its recorded time: a pass
/// recorded with an earlier clock after a fail still satisfies the gate.
#[test]
fn receipt_order_is_seq_not_the_clock() -> TestResult {
    let owned = world(&[])?;
    let w = &owned;
    // An observed gate whose result depends on a file outside its scope:
    // both receipts are about the same definition and bytes.
    let gates = w.kb.join(".writfold/gates");
    put(
        &gates.join("flaky.toml"),
        "schema = \"writfold-gate/1\"\nid = \"flaky\"\nversion = 1\nclass = \"observed\"\nargv = [\"test\", \"-e\", \"../flag\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n",
    )?;
    let mut request = w.request(Verb::Gate(GateVerb::Run {
        id: "flaky".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }));
    request.now = Some(NOW);
    assert_eq!(w.run(&request)?.1, 2, "fails while the flag is absent");
    put(&w.base.join("flag"), "")?;
    request.now = Some(NOW - 86_400);
    assert_eq!(w.run(&request)?.1, 0, "passes, with a clock a day behind");
    let status = Verb::Gate(GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let (text, _) = call(w, status)?;
    assert!(text.contains("gate flaky [satisfied]"), "{text}");
    Ok(())
}

/// The fold is a function of the set of event files: every rotation and
/// reversal of the listing gives the same steps, states and invalid list,
/// and a clone of the KB answers `plan status` with the same rows.
#[test]
fn the_fold_depends_only_on_the_set_of_files() -> TestResult {
    let owned = world(&[])?;
    let w = &owned;
    define(w)?;
    plan(w, "set")?;
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-301".into(),
        title: "Backfill".into(),
        ..Ticket::default()
    });
    call(w, Verb::Write(ticket))?;
    gate(w, "backfilled")?;
    plan(w, "advance")?;
    let skip = PlanVerb::Skip {
        reason: "covered by CI".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    assert_eq!(call(w, Verb::Plan(skip))?.1, 0);
    // A stray, hand-written event is listed as invalid wherever it sorts.
    put(
        &w.kb
            .join(format!("runs/{RUN}/plan/done-change.1~review.json")),
        "{}\n",
    )?;
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let mut files: Vec<(String, String)> = Vec::new();
    for name in events(w) {
        if name.ends_with(".json") {
            files.push((name.clone(), std::fs::read_to_string(dir.join(&name))?));
        }
    }
    let snapshot = |digest: &str| {
        let hex = digest.strip_prefix("sha256:")?;
        std::fs::read(dir.join("templates").join(format!("{hex}.toml"))).ok()
    };
    let shape = |files: Vec<(String, String)>| {
        let g = writfold_kb::plan::fold::fold(RUN, "x", files, &snapshot);
        let steps: Vec<String> = g
            .steps
            .iter()
            .map(|s| {
                format!(
                    "{} {} {} {:?}",
                    s.path,
                    s.done.is_some(),
                    s.skipped.is_some(),
                    s.begin
                )
            })
            .collect();
        let current = g.current();
        (steps, g.invalid, current)
    };
    let expected = shape(files.clone());
    assert_eq!(expected.2, Some(2), "{expected:?}");
    assert_eq!(expected.1.len(), 1);
    for turn in 0..files.len() {
        let mut turned = files.clone();
        turned.rotate_left(turn);
        assert_eq!(shape(turned.clone()), expected);
        turned.reverse();
        assert_eq!(shape(turned), expected);
    }
    same_rows_in_a_clone(w)
}

/// A clone of the KB, with the same files, answers with the same rows.
fn same_rows_in_a_clone(w: &World) -> TestResult {
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "plan"])?;
    let clone = w.base.join("clone");
    git(
        &w.base,
        &["clone", "-q", &w.kb.display().to_string(), "clone"],
    )?;
    std::fs::create_dir_all(clone.join(".git/writfold"))?;
    std::fs::copy(
        w.kb.join(".git/writfold/repos"),
        clone.join(".git/writfold/repos"),
    )?;
    let rows = |kb: &Path| -> TestResult<Vec<String>> {
        let mut request = w.request(Verb::Plan(PlanVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }));
        request.kb = Some(kb.to_path_buf());
        let out = writfold_kb::run(&request)?;
        Ok(out
            .text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect())
    };
    assert_eq!(rows(&clone)?, rows(&w.kb)?);
    Ok(())
}
