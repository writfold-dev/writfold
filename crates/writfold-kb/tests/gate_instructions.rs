//! Instruction gates on repositories: a gate may name an `instructions`
//! file (shown as a `read:` line by `gate status`, `gate run` and
//! `resume`) and ship its own script through `{gate}`. Both files' bytes
//! are part of the definition digest, so editing them makes receipts and
//! attestations stale; a comment in the TOML does not. Paths that escape
//! the definition's directory are refused.
mod support;

use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const LINT: &str = "schema = \"writfold-gate/1\"\nid = \"lint\"\nversion = 1\nclass = \"mechanical\"\ninstructions = \"lint.md\"\nargv = [\"sh\", \"{gate}/lint.sh\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n";
const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\ninstructions = \"review/checklist.md\"\nscope = [\"src/**\"]\n";

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

/// The KB's `lint` and `review` gates with their files.
fn define(w: &World) -> TestResult<PathBuf> {
    let gates = w.kb.join(".writfold/gates");
    put(&gates.join("lint.toml"), LINT)?;
    put(
        &gates.join("lint.md"),
        "# Lint\n\nEvery source file is under src/.\n",
    )?;
    // The script runs in the subject repository: it judges that tree.
    put(
        &gates.join("lint.sh"),
        "test -d src && ! grep -rq FIXME src\n",
    )?;
    put(&gates.join("review.toml"), REVIEW)?;
    put(
        &gates.join("review/checklist.md"),
        "- Read the whole diff.\n",
    )?;
    Ok(gates)
}

fn gate(w: &World, verb: GateVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(at);
    w.run(&request)
}

fn run(w: &World, id: &str, repo: &Path, at: i64) -> TestResult<(String, u8)> {
    let verb = GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(repo.to_path_buf()),
    };
    gate(w, verb, at)
}

fn status(w: &World, repo: &Path) -> TestResult<(String, u8)> {
    let verb = GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(repo.to_path_buf()),
    };
    gate(w, verb, NOW + 3600)
}

/// The standing of one gate, from a status answer.
fn state_of(text: &str, id: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("gate {id} [")))
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("absent")
        .to_owned()
}

#[test]
fn instructions_are_read_lines_and_their_edits_make_receipts_stale() -> TestResult {
    let w = world(&[])?;
    let gates = define(&w)?;
    let (text, exit) = run(&w, "lint", &w.code, NOW)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("  read: instructions for lint @.writfold/gates/lint.md:1"),
        "{text}"
    );
    let (text, exit) = status(&w, &w.code)?;
    golden("gate-status-instructions.txt", &text, exit)?;
    let mut request = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    let (resumed, _) = w.run(&request)?;
    assert!(
        resumed.contains("  read: instructions for review @.writfold/gates/review/checklist.md:1"),
        "{resumed}"
    );
    // The attested gate is met by an attestation, never satisfied.
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "review".into(),
        text: "read the diff against the checklist".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    gate(&w, say, NOW + 60)?;
    let (text, exit) = status(&w, &w.code)?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(state_of(&text, "review"), "attested");
    assert!(
        text.contains("satisfied=1") && text.contains("attested=1"),
        "{text}"
    );
    // A comment in the definition leaves the receipt current.
    put(
        &gates.join("lint.toml"),
        &format!("# lint, reviewed\n{LINT}"),
    )?;
    assert_eq!(state_of(&status(&w, &w.code)?.0, "lint"), "satisfied");
    // Editing the instructions makes the receipt stale, and says why.
    put(
        &gates.join("lint.md"),
        "# Lint\n\nNo FIXME is left in src/.\n",
    )?;
    let (text, exit) = status(&w, &w.code)?;
    assert_eq!(exit, 2, "{text}");
    assert_eq!(state_of(&text, "lint"), "stale");
    assert!(
        text.contains("why: the definition changed since this receipt"),
        "{text}"
    );
    assert_eq!(state_of(&text, "review"), "attested");
    put(
        &gates.join("review/checklist.md"),
        "- Read the whole diff twice.\n",
    )?;
    assert_eq!(state_of(&status(&w, &w.code)?.0, "review"), "stale");
    // Editing the shipped script does too.
    run(&w, "lint", &w.code, NOW + 120)?;
    put(&gates.join("lint.sh"), "test -d src\n")?;
    assert_eq!(state_of(&status(&w, &w.code)?.0, "lint"), "stale");
    Ok(())
}

/// A second repository, with or without a FIXME in its source.
fn other_repo(w: &World, name: &str, fixme: bool) -> TestResult<PathBuf> {
    let dir = w.base.join(name);
    let body = if fixme {
        "// FIXME\n"
    } else {
        "pub fn x() {}\n"
    };
    put(&dir.join("src/lib.rs"), body)?;
    git(&dir, &["init", "-q"])?;
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-q", "-m", "first"])?;
    Ok(dir)
}

#[test]
fn a_kb_gate_runs_its_own_script_against_two_repositories() -> TestResult {
    let w = world(&[])?;
    define(&w)?;
    let clean = other_repo(&w, "clean-service", false)?;
    let marked = other_repo(&w, "marked-service", true)?;
    assert_eq!(run(&w, "lint", &w.code, NOW)?.1, 0);
    assert_eq!(run(&w, "lint", &clean, NOW + 60)?.1, 0);
    let (text, exit) = run(&w, "lint", &marked, NOW + 120)?;
    assert!(exit == 2 && text.contains("gate lint [failed]"), "{text}");
    for (repo, want) in [
        (&w.code, "satisfied"),
        (&clean, "satisfied"),
        (&marked, "failed"),
    ] {
        assert_eq!(
            state_of(&status(&w, repo)?.0, "lint"),
            want,
            "{}",
            repo.display()
        );
    }
    // A repository's own gate ships its script beside its definition;
    // `gate verify` finds it in the worktree at the recorded commit.
    let local = "schema = \"writfold-gate/1\"\nid = \"local\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"sh\", \"{gate}/local.sh\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n";
    put(&clean.join(".writfold/gates/local.toml"), local)?;
    put(
        &clean.join(".writfold/gates/local.sh"),
        "test -f src/lib.rs\n",
    )?;
    git(&clean, &["add", "-A"])?;
    git(&clean, &["commit", "-q", "-m", "local gate"])?;
    assert_eq!(run(&w, "local", &clean, NOW + 180)?.1, 0);
    let receipts = w.kb.join(format!("runs/{RUN}/gates"));
    let mut names: Vec<String> = std::fs::read_dir(&receipts)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("local-"))
        .collect();
    names.sort();
    let given = format!("runs/{RUN}/gates/{}", names.first().ok_or("no receipt")?);
    let verb = GateVerb::Verify {
        receipt: given,
        repo: Some(clean.clone()),
    };
    let (text, exit) = gate(&w, verb, NOW + 240)?;
    assert!(exit == 0 && text.contains("[agree]"), "{text}");
    Ok(())
}

#[test]
fn paths_that_escape_the_definition_directory_are_refused() -> TestResult {
    let w = world(&[])?;
    let gates = w.kb.join(".writfold/gates");
    put(&w.kb.join(".writfold/outside.md"), "outside\n")?;
    put(&w.kb.join(".writfold/outside.sh"), "exit 0\n")?;
    let base = "schema = \"writfold-gate/1\"\nversion = 1\nclass = \"mechanical\"\ntimeout_seconds = 30\nscope = [\"src/**\"]\n";
    let cases = [
        (
            "up",
            "instructions = \"../outside.md\"\nargv = [\"true\"]\n",
            "instructions",
        ),
        (
            "script",
            "argv = [\"sh\", \"{gate}/../outside.sh\"]\n",
            "{gate}",
        ),
        (
            "absolute",
            "instructions = \"/etc/hostname.md\"\nargv = [\"true\"]\n",
            "instructions",
        ),
        (
            "linked",
            "instructions = \"linked.md\"\nargv = [\"true\"]\n",
            "escapes",
        ),
        (
            "draft",
            "argv = [\"cat\", \"{candidate}\"]\n",
            "{candidate}",
        ),
    ];
    std::fs::create_dir_all(&gates)?;
    std::os::unix::fs::symlink(w.kb.join(".writfold/outside.md"), gates.join("linked.md"))?;
    for (id, extra, _) in cases {
        put(
            &gates.join(format!("{id}.toml")),
            &format!("id = \"{id}\"\n{base}{extra}"),
        )?;
    }
    let (text, exit) = status(&w, &w.code)?;
    assert_eq!(exit, 2, "{text}");
    for (id, _, why) in cases {
        let row = text
            .lines()
            .find(|l| {
                l.starts_with(&format!(
                    "gate-definition .writfold/gates/{id}.toml [invalid]"
                ))
            })
            .ok_or_else(|| format!("no invalid row for {id}:\n{text}"))?;
        assert!(row.contains(why), "{id}: {row}");
        let refused = writfold_kb::run(&w.request(Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        })));
        assert!(refused.is_err(), "{id} ran");
    }
    Ok(())
}
