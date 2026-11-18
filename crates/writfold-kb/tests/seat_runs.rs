//! A current run per session and per work tree (`writfold_kb::seat`):
//! parallel sessions on one KB clone are each judged against their own
//! run, the most specific pointer that is set decides, an ended run there
//! is reported and never skipped, and without the narrower pointers the
//! clone's shared pointer answers as before.
mod support;

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, git, world};
use writfold_kb::gate::GateVerb;
use writfold_kb::guard::GuardVerb;
use writfold_kb::hook::{HookEnv, HookOutput};
use writfold_kb::seat::{self, Seat};
use writfold_kb::write::Write;
use writfold_kb::write::run::{Checkpoint, NewRun};
use writfold_kb::write::status::SetStatus;
use writfold_kb::{Request, Verb};

const FIXTURE: &str = "run-20261101T090000Z-a1b2c3";
const COMMIT: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"run.current\", \"gate:unit\"]\naction = \"deny\"\n";
const NEED_RUN: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"run.current\"]\naction = \"deny\"\n";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn state(w: &World) -> PathBuf {
    w.kb.join(".git/writfold")
}

/// A second work repository beside the fixture's.
fn other_repo(w: &World) -> TestResult<PathBuf> {
    let dir = w.base.join("other-service");
    put(&dir.join("src/ledger.rs"), "pub fn write() {}\n")?;
    git(&dir, &["init", "-q"])?;
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-q", "-m", "first"])?;
    Ok(dir)
}

fn runs(w: &World) -> TestResult<Vec<String>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(w.kb.join("runs"))? {
        found.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(found)
}

/// Start a run from `cwd`; its ID.
fn start(w: &World, cwd: &Path, task: &str) -> TestResult<String> {
    let before = runs(w)?;
    let mut request = w.request(Verb::Write(Write::Checkpoint(Checkpoint {
        artifacts: Vec::new(),
        run: None,
        new_run: Some(NewRun {
            project: "harbor".into(),
            task: task.into(),
            mode: None,
        }),
        note: format!("Start: {task}."),
    })));
    request.cwd = cwd.to_path_buf();
    let output = writfold_kb::run(&request)?;
    assert_eq!(output.exit, 0, "{}", output.text);
    let new = runs(w)?.into_iter().find(|id| !before.contains(id));
    new.ok_or_else(|| "no new run".into())
}

fn request_at(w: &World, cwd: &Path, verb: Verb) -> Request {
    let mut request = w.request(verb);
    request.cwd = cwd.to_path_buf();
    request
}

fn pass_unit(w: &World, run: &str, repo: &Path) -> TestResult {
    let verb = Verb::Gate(GateVerb::Run {
        id: "unit".into(),
        run: Some(run.into()),
        repo: Some(repo.to_path_buf()),
    });
    let output = writfold_kb::run(&w.request(verb))?;
    assert_eq!(output.exit, 0, "{}", output.text);
    Ok(())
}

/// The guard hook before `git commit` in `cwd`, for `session`.
fn commit_hook(w: &World, cwd: &Path, session: &str) -> HookOutput {
    let input = json!({"session_id": session, "cwd": cwd, "hook_event_name": "PreToolUse",
        "tool_name": "Bash", "tool_input": {"command": "git commit -m x"}});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 60),
    };
    writfold_kb::hook::run("pre-tool-use", &input.to_string(), &env)
}

fn denial(output: &HookOutput) -> TestResult<String> {
    let value: Value = serde_json::from_str(&output.stdout)?;
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["permissionDecision"], "deny", "{value}");
    Ok(specific["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?
        .to_owned())
}

/// The run `guard status` judges for in `cwd`, and its notes.
fn judged_for(w: &World, cwd: &Path) -> TestResult<(String, String)> {
    let mut request = request_at(w, cwd, Verb::Guard(GuardVerb::Status));
    request.json = true;
    let output = writfold_kb::run(&request)?;
    let value: Value = serde_json::from_str(&output.text)?;
    let notes = value["notes"].to_string();
    let run = notes
        .split("judged for the current run ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .ok_or("no judged note")?;
    Ok((run.to_owned(), notes))
}

/// The failure this exists for: two sessions on one clone, each in its
/// own repository on its own task. Starting the second run moves the
/// shared pointer; the first session is still judged against its own run.
#[test]
fn sessions_in_two_repositories_keep_their_own_run() -> TestResult {
    let w = world(&[])?;
    let other = other_repo(&w)?;
    put(&w.kb.join(".writfold/guards/commit.toml"), COMMIT)?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    let first = start(&w, &w.code, "Close HBR-101")?;
    pass_unit(&w, &first, &w.code)?;
    let before = commit_hook(&w, &w.code, "session-a");
    assert!(before.stdout.is_empty(), "{}", before.stdout);

    let second = start(&w, &other, "Close HBR-102")?;
    let shared = std::fs::read_to_string(state(&w).join("current-run"))?;
    assert_eq!(
        shared.trim(),
        second,
        "the shared pointer is the last run made current"
    );

    // Session A, in its own repository, is judged against its own run.
    let after = commit_hook(&w, &w.code, "session-a");
    assert!(after.stdout.is_empty(), "{}", after.stdout);
    // Session B owes the gate in its run.
    let reason = denial(&commit_hook(&w, &other, "session-b"))?;
    assert!(reason.contains("gate unit [open]"), "{reason}");

    // `guard status` reads the same pointers as the hook, and says which.
    let (run, notes) = judged_for(&w, &w.code)?;
    assert_eq!(run, first);
    assert!(
        notes.contains("comes from the tree pointer (work tree "),
        "{notes}"
    );
    assert_eq!(judged_for(&w, &other)?.0, second);
    // Outside both repositories only the shared pointer is left.
    let (run, notes) = judged_for(&w, &w.base)?;
    assert_eq!(run, second);
    assert!(!notes.contains("comes from the"), "{notes}");

    // Without the work-tree pointers every seat falls back to the shared
    // pointer: session A is then judged against the other session's run.
    std::fs::remove_dir_all(state(&w).join(seat::DIR))?;
    let reason = denial(&commit_hook(&w, &w.code, "session-a"))?;
    assert!(reason.contains("gate unit [open]"), "{reason}");
    Ok(())
}

/// Two sessions in one work tree: the session's own pointer decides before
/// the work tree's.
#[test]
fn a_session_pointer_decides_before_the_work_tree() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/commit.toml"), COMMIT)?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    let first = start(&w, &w.code, "Close HBR-101")?;
    pass_unit(&w, &first, &w.code)?;
    // What `checkpoint --new-run` does for a session whose ID it knows.
    let seat_a = Seat {
        session: Some("session-a".into()),
        tree: Some(w.code.clone()),
    };
    seat::make_current(&state(&w), &seat_a, &first)?;
    // A second session starts its own run in the same work tree.
    let second = start(&w, &w.code, "Close HBR-102")?;
    assert_ne!(first, second);
    assert!(commit_hook(&w, &w.code, "session-a").stdout.is_empty());
    let reason = denial(&commit_hook(&w, &w.code, "session-b"))?;
    assert!(reason.contains("gate unit [open]"), "{reason}");
    // The session's pointer follows it into another directory.
    assert!(commit_hook(&w, &w.base, "session-a").stdout.is_empty());
    Ok(())
}

fn close(w: &World, run: &str) -> TestResult {
    let set = SetStatus {
        target: run.into(),
        state: "closed".into(),
        by: None,
        superseded_by: None,
        note: None,
        commit: None,
        evidence: None,
        run: None,
        repo: None,
    };
    let output = writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(set))))?;
    assert_eq!(output.exit, 0, "{}", output.text);
    Ok(())
}

/// A session whose run ended is told so. It is not moved onto the open run
/// a less specific pointer names (another session's).
#[test]
fn an_ended_run_is_reported_not_skipped() -> TestResult {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/commit.toml"), NEED_RUN)?;
    let mine = start(&w, &w.code, "Close HBR-101")?;
    let seat_a = Seat {
        session: Some("session-a".into()),
        tree: None,
    };
    seat::make_current(&state(&w), &seat_a, &mine)?;
    let theirs = start(&w, &w.code, "Close HBR-102")?;
    assert!(commit_hook(&w, &w.code, "session-a").stdout.is_empty());

    close(&w, &mine)?;
    let reason = denial(&commit_hook(&w, &w.code, "session-a"))?;
    assert!(
        reason.contains(&format!("run {mine} is closed")),
        "{reason}"
    );
    assert!(!reason.contains(&theirs), "{reason}");
    // The other session, on the work tree's pointer, is untouched.
    assert!(commit_hook(&w, &w.code, "session-b").stdout.is_empty());

    // A pointer naming a run the KB lacks is reported the same way.
    seat::make_current(&state(&w), &seat_a, "run-gone")?;
    let reason = denial(&commit_hook(&w, &w.code, "session-a"))?;
    assert!(
        reason.contains("run-gone, which is not a run in the KB"),
        "{reason}"
    );
    Ok(())
}

/// `resume <run> --here` points this seat at an open run without writing
/// a tracked file; a closed run is refused.
#[test]
fn resume_here_makes_an_open_run_current() -> TestResult {
    let w = world(&[])?;
    let other = other_repo(&w)?;
    let first = start(&w, &w.code, "Close HBR-101")?;
    let dirty = git(&w.kb, &["status", "--porcelain"])?;
    let attach = |run: &str| request_at(&w, &other, Verb::Attach { run: run.into() });

    // A reviewer working in another repository attaches to the run.
    assert_eq!(
        judged_for(&w, &other)?.0,
        first,
        "only the shared pointer so far"
    );
    let (text, exit) = w.run(&attach(FIXTURE))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains(&format!("{FIXTURE} is now the current run here")),
        "{text}"
    );
    assert_eq!(judged_for(&w, &other)?.0, FIXTURE);
    // The first repository keeps its run; nothing tracked was written.
    assert_eq!(judged_for(&w, &w.code)?.0, first);
    assert_eq!(git(&w.kb, &["status", "--porcelain"])?, dirty);

    close(&w, &first)?;
    let refused = writfold_kb::run(&attach(&first));
    let message = refused.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(message.contains("is closed"), "{message}");
    assert_eq!(
        judged_for(&w, &other)?.0,
        FIXTURE,
        "a refusal moves no pointer"
    );
    Ok(())
}

/// The KB's own work tree has no pointer of its own: a run started there
/// is the shared pointer's, as before.
#[test]
fn the_kb_work_tree_uses_the_shared_pointer() -> TestResult {
    let w = world(&[])?;
    let id = start(&w, &w.kb, "Tidy the KB")?;
    assert_eq!(
        std::fs::read_to_string(state(&w).join("current-run"))?.trim(),
        id
    );
    assert!(!state(&w).join(seat::DIR).exists());
    let (run, notes) = judged_for(&w, &w.kb)?;
    assert_eq!(run, id);
    assert!(!notes.contains("comes from the"), "{notes}");
    Ok(())
}

const GATES_MET: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"run.current\", \"gates.met\"]\naction = \"deny\"\n";
const PY_PLAN: &str =
    "schema = \"writfold-plan/1\"\nname = \"py-change\"\nsteps = [\n  \"implement: py-unit\",\n]\n";

/// Gates in the KB root apply to every repository, but a run's plan
/// narrows `gates.met` to the gates its steps name: a run on a plan that
/// names only `py-unit` owes no `cpp-unit`, while a run without a plan in
/// the same repository owes both.
#[test]
fn a_plan_narrows_the_kb_root_gates_a_run_owes() -> TestResult {
    use writfold_kb::plan::PlanVerb;
    let w = world(&[])?;
    put(&w.kb.join(".writfold/guards/commit.toml"), GATES_MET)?;
    for id in ["py-unit", "cpp-unit"] {
        let text = UNIT.replace("\"unit\"", &format!("\"{id}\""));
        put(&w.kb.join(format!(".writfold/gates/{id}.toml")), &text)?;
    }
    put(&w.kb.join(".writfold/plans/py-change.toml"), PY_PLAN)?;
    let run = start(&w, &w.code, "A Python change")?;
    let set = Verb::Plan(PlanVerb::Set {
        template: Some("py-change".into()),
        from: None,
        run: Some(run.clone()),
        repo: Some(w.code.clone()),
    });
    let output = writfold_kb::run(&w.request(set))?;
    assert_eq!(output.exit, 0, "{}", output.text);
    let reason = denial(&commit_hook(&w, &w.code, "session-a"))?;
    assert!(reason.contains("gate py-unit [open]"), "{reason}");
    assert!(!reason.contains("cpp-unit"), "{reason}");
    let pass = Verb::Gate(GateVerb::Run {
        id: "py-unit".into(),
        run: Some(run),
        repo: Some(w.code.clone()),
    });
    assert_eq!(writfold_kb::run(&w.request(pass))?.exit, 0);
    let passed = commit_hook(&w, &w.code, "session-a");
    assert!(passed.stdout.is_empty(), "{}", passed.stdout);

    // A run with no plan owes every gate defined for the repository.
    start(&w, &w.code, "No plan")?;
    let reason = denial(&commit_hook(&w, &w.code, "session-b"))?;
    assert!(reason.contains("gate py-unit [open]"), "{reason}");
    assert!(reason.contains("gate cpp-unit [open]"), "{reason}");
    Ok(())
}
