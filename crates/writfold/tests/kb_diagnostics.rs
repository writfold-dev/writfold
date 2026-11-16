//! Diagnostics through the real binary, on a scratch KB with one broken
//! file each: the refusal draws every fault on stderr and still ends with
//! the one `writfold kb: refused: …` line; `gate status` and `guard
//! status` keep their `[invalid]` row and carry `diagnostics` in JSON;
//! `kb lint` exits 0, 2, and 2 under `--deny-warnings`; a hook exits 0
//! and names the fault in one compact line.
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");
const RUN: &str = "run-20261101T090000Z-a1b2c3";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"cargo\", \"test\"]\ntimeout_seconds = 600\nscope = [\"src/**\"]\n";

struct World(PathBuf);

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

fn copy(from: &Path, to: &Path) -> Result {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        match entry.file_type()?.is_dir() {
            true => copy(&entry.path(), &to.join(entry.file_name()))?,
            false => {
                fs::copy(entry.path(), to.join(entry.file_name()))?;
            }
        }
    }
    Ok(())
}

/// The basic fixture KB with a current run, and a subject repository.
fn world(name: &str, files: &[(&str, &str)]) -> Result<World> {
    let base = std::env::temp_dir().join(format!("writfold-diag-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kb/basic");
    copy(&fixture, &base.join("kb"))?;
    fs::create_dir_all(base.join("demo/src"))?;
    fs::write(base.join("demo/src/main.rs"), "fn main() {}\n")?;
    for dir in ["demo", "kb"] {
        git(&base.join(dir), &["init", "-q"])?;
        git(&base.join(dir), &["add", "-A"])?;
        git(&base.join(dir), &["commit", "-qm", "first"])?;
    }
    fs::create_dir_all(base.join("kb/.git/writfold"))?;
    fs::write(base.join("kb/.git/writfold/current-run"), RUN)?;
    for (path, text) in files {
        let path = base.join("kb").join(path);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, text)?;
    }
    Ok(World(fs::canonicalize(base)?))
}

fn kb(w: &World, args: &[&str], stdin: Option<&str>) -> Result<(String, String, i32)> {
    let mut child = Command::new(WRITFOLD)
        .current_dir(w.0.join("demo"))
        .env_remove("WRITFOLD_KB")
        .env("WRITFOLD_KB", w.0.join("kb"))
        .env("NO_COLOR", "1")
        .arg("kb")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(stdin.unwrap_or("").as_bytes())?;
    }
    let out = child.wait_with_output()?;
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    Ok((
        text(&out.stdout),
        text(&out.stderr),
        out.status.code().unwrap_or(-1),
    ))
}

#[test]
fn a_refusal_draws_every_fault_and_ends_with_the_refused_line() -> Result {
    let broken = UNIT
        .replace("version = 1", "version = \"one\"")
        .replace("timeout_seconds = 600\n", "");
    let w = world("refusal", &[(".writfold/gates/unit.toml", &broken)])?;
    let (stdout, stderr, code) = kb(&w, &["gate", "run", "unit"], None)?;
    assert_eq!((stdout.as_str(), code), ("", 3), "{stderr}");
    assert!(stderr.starts_with("error[key.type]: `version` must be an integer, and this is a string\n --> .writfold/gates/unit.toml:3:11\n"), "{stderr}");
    assert!(
        stderr.contains("error[key.missing]: a mechanical gate needs `timeout_seconds`"),
        "{stderr}"
    );
    assert_eq!(
        stderr.lines().last(),
        Some("writfold kb: refused: .writfold/gates/unit.toml: 2 errors")
    );
    let (stdout, stderr, code) = kb(&w, &["gate", "status"], None)?;
    assert_eq!(code, 2, "{stdout}");
    let row = stdout
        .lines()
        .find(|l| l.starts_with("gate-definition"))
        .ok_or("no row")?;
    assert_eq!(
        row,
        "gate-definition .writfold/gates/unit.toml [invalid] `version` must be an integer, and this is a string @.writfold/gates/unit.toml:3 !invalid"
    );
    assert!(
        stdout.contains("  next: writfold kb lint .writfold/gates/unit.toml"),
        "{stdout}"
    );
    assert!(stderr.contains("error[key.type]"), "{stderr}");
    let (stdout, _, _) = kb(&w, &["gate", "status", "--json"], None)?;
    let body: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        body["diagnostics"].as_array().map(Vec::len),
        Some(2),
        "{body}"
    );
    assert_eq!(body["diagnostics"][1]["line"], Value::Null);
    Ok(())
}

#[test]
fn guard_status_names_the_rule_by_its_file_and_the_hook_says_it_is_not_guarding() -> Result {
    let rule = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gates.mett\"]\naction = \"deny\"\n";
    let w = world("guard", &[(".writfold/guards/commit.toml", rule)])?;
    let (stdout, _, code) = kb(&w, &["guard", "status"], None)?;
    assert_eq!(code, 2);
    assert!(stdout.contains("guard-rule commit [invalid] unknown requirement \"gates.mett\" @.writfold/guards/commit.toml:5 !invalid\n  help: did you mean \"gates.met\"?\n"), "{stdout}");
    let input =
        "{\"hook_event_name\":\"SessionStart\",\"source\":\"startup\",\"session_id\":\"s\"}";
    let (stdout, _, code) = kb(&w, &["hook", "session-start"], Some(input))?;
    assert_eq!(code, 0);
    let body: Value = serde_json::from_str(&stdout)?;
    let context = body["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap_or("");
    assert!(context.contains(".writfold/guards/commit.toml:5:12: unknown requirement \"gates.mett\"; did you mean \"gates.met\"? This rule is not guarding."), "{context}");
    let input = "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"git commit -m x\"},\"session_id\":\"s\"}";
    let (stdout, stderr, code) = kb(&w, &["hook", "pre-tool-use"], Some(input))?;
    assert_eq!((stdout.as_str(), code), ("", 0));
    assert!(
        stderr.contains("guard rule skipped: .writfold/guards/commit.toml:5:12:"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_plan_names_an_undefined_gate_at_its_word() -> Result {
    let plan =
        "schema = \"writfold-plan/1\"\nname = \"main\"\nsteps = [\n  \"implement: unitt\",\n]\n";
    let w = world(
        "plan",
        &[
            (".writfold/gates/unit.toml", UNIT),
            (".writfold/plans/main.toml", plan),
        ],
    )?;
    let (_, stderr, code) = kb(&w, &["plan", "set", "main"], None)?;
    assert_eq!(code, 3, "{stderr}");
    assert!(
        stderr.contains(" --> .writfold/plans/main.toml:4:15\n")
            && stderr.contains("= help: did you mean `unit`?"),
        "{stderr}"
    );
    assert_eq!(
        stderr.lines().last(),
        Some("writfold kb: refused: .writfold/plans/main.toml: 1 error")
    );
    Ok(())
}

#[test]
fn lint_exits_zero_two_and_two_under_deny_warnings() -> Result {
    let mode = "schema = \"writfold-delivery-mode/1\"\nmode = \"note2\"\nattest = [\"read\"]\n";
    let plan = "schema = \"writfold-plan/1\"\nname = \"ship\"\nsteps = [\"x: deliver:note2\"]\n";
    let files = [
        (".writfold/gates/unit.toml", UNIT),
        (".writfold/deliveries/note2.toml", mode),
        (".writfold/plans/ship.toml", plan),
    ];
    let w = world("lint", &files)?;
    let (stdout, _, code) = kb(&w, &["lint"], None)?;
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        "# coverage=complete files=3 clean=3 invalid=0 errors=0 warnings=0\n"
    );
    let rule = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gate:unitt\"]\naction = \"deny\"\n";
    fs::create_dir_all(w.0.join("kb/.writfold/guards"))?;
    fs::write(w.0.join("kb/.writfold/guards/commit.toml"), rule)?;
    // Outside any repository, a KB-root rule naming an unknown gate is a warning.
    let elsewhere = w.0.join("elsewhere");
    fs::create_dir_all(&elsewhere)?;
    let run = |args: &[&str]| -> Result<(String, i32)> {
        let out = Command::new(WRITFOLD)
            .current_dir(&elsewhere)
            .env("WRITFOLD_KB", w.0.join("kb"))
            .arg("kb")
            .args(args)
            .output()?;
        Ok((
            String::from_utf8_lossy(&out.stdout).into_owned(),
            out.status.code().unwrap_or(-1),
        ))
    };
    let (stdout, code) = run(&["lint"])?;
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.starts_with("warning[ref.undefined]: rule `commit` owes gate `unitt`"),
        "{stdout}"
    );
    assert_eq!(run(&["lint", "--deny-warnings"])?.1, 2);
    let (stdout, _, code) = kb(&w, &["lint", "--json"], None)?;
    assert_eq!(code, 2, "{stdout}");
    let body: Value = serde_json::from_str(&stdout)?;
    assert_eq!(body["diagnostics"][0]["severity"], "error");
    assert_eq!(body["trailer"]["errors"], 1);
    assert_eq!(kb(&w, &["lint", "no/such/file.toml"], None)?.2, 1);
    Ok(())
}

#[test]
fn a_step_owing_a_gate_that_stopped_parsing_says_why() -> Result {
    let plan = "schema = \"writfold-plan/1\"\nname = \"main\"\nsteps = [\"implement: unit\"]\n";
    let files = [
        (".writfold/gates/unit.toml", UNIT),
        (".writfold/plans/main.toml", plan),
    ];
    let w = world("owed", &files)?;
    let (_, stderr, code) = kb(&w, &["plan", "set", "main"], None)?;
    assert_eq!(code, 0, "{stderr}");
    let broken = UNIT.replace("timeout_seconds", "timout_seconds");
    fs::write(w.0.join("kb/.writfold/gates/unit.toml"), broken)?;
    let (stdout, _, _) = kb(&w, &["plan", "status", "--gates"], None)?;
    assert!(
        stdout.contains("  reason: .writfold/gates/unit.toml cannot be read: line 6: unknown key timout_seconds in a gate definition"),
        "{stdout}"
    );
    Ok(())
}
