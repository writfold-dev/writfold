//! `trial run` end to end with a stub Weft binary: the template resolves to
//! the exact retained binding, an unresolved template refuses before Weft
//! runs, each failure names its step, and a refuted check makes a later
//! `approve --require` refuse.
use super::{Request, render, run, weft::sha256_hex};
use crate::trial::retain::retained;
use crate::trial::{Result, Trial, approve};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const RECIPE: &str =
    "edit \"src/lib.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";
/// Emits a fixed `weft.report/1` citing the first `citation` reference of the
/// source it was given, and keeps a copy of that source as `seen.weft`.
const STUB: &str = r#"#!/bin/sh
[ "$1 $2 $3" = "prove --json --" ] || exit 9
cp "$4" "$(dirname "$0")/seen.weft"
ref=$(sed -n 's/.*citation "\([^"]*\)".*/\1/p' "$4" | head -n 1)
printf '{"schema":"weft.report/1","kind":"assessment","scope":"local-symbolic","execution":"complete","writfold_grounded":false,"effect_authority":false,"review":{"bindings":[{"kind":"asset","reference":"%s"}]},"checks":[{"name":"ok","result":"entailed"},{"name":"bad","result":"refuted"}]}' "$ref"
exit 1
"#;

struct Fixture {
    root: PathBuf,
    state: PathBuf,
    repo: PathBuf,
    commit: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sh(script: &str, args: &[&str]) -> Result<String> {
    let out = Command::new("sh")
        .args(["-c", script, "sh"])
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .output()?;
    if !out.status.success() {
        return Err(format!("{script} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
}

/// Written by a child process, so no descriptor of this (multi-threaded)
/// process holds the file open when it is executed.
fn executable(path: &Path, body: &str) -> Result<()> {
    let path = path.to_str().ok_or("path")?;
    sh(
        r#"printf '%s' "$1" > "$2" && chmod 755 "$2""#,
        &[body, path],
    )?;
    Ok(())
}

fn fixture() -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-trial-run-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/lib.rs"), "let a = 1;\nrun(1);\n")?;
    let dir = repo.to_str().ok_or("path")?;
    sh(
        r#"cd "$1" && git init -q && git add -A && git commit -qm fixture"#,
        &[dir],
    )?;
    let commit = sh(r#"git -C "$1" rev-parse HEAD"#, &[dir])?;
    let state = root.join("state");
    Trial::init(&state)?;
    fs::write(root.join("edit.spl"), RECIPE)?;
    fs::write(
        root.join("q.tuft"),
        format!(
            "@h := find text \"run(\" at \"{commit}\";\n@e := retain @h;\n\
@v := project @e [path, line, binding];\nemit @v;\n"
        ),
    )?;
    fs::create_dir(root.join("weft-bin"))?;
    executable(&root.join("weft-bin/weft"), STUB)?;
    Ok(Fixture {
        root,
        state,
        repo,
        commit,
    })
}

fn review(f: &Fixture, citation: &str) -> Result<PathBuf> {
    let path = f.root.join("review.weft");
    fs::write(
        &path,
        format!(
            "review t {{\n    context \"trial\" at \"{}\";\n    profile \"weft.symbolic/1\";\n    \
target change = proposal \"run-2\";\n    asset call = citation \"{citation}\";\n    \
assume a: ok(\"x\");\n    check ok: ok(\"x\");\n}}\n",
            f.commit
        ),
    )?;
    Ok(path)
}

fn trial_run(f: &Fixture, tuft: &Path, review: Option<&Path>, bin: &Path) -> Result<super::Ran> {
    run(&Request {
        state: &f.state,
        repo: &f.repo,
        file: "src/lib.rs",
        recipe: &f.root.join("edit.spl"),
        tuft: Some(tuft),
        weft: review.map(|r| (r, bin)),
        weft_sha256: None,
    })
}

fn no_worktrees(f: &Fixture) -> bool {
    fs::read_dir(f.state.join("worktrees")).map_or(true, |mut d| d.next().is_none())
}

#[test]
fn one_command_binds_evidence_report_and_prepared_edit() -> Result<()> {
    let f = fixture()?;
    let bin = f.root.join("weft-bin/weft");
    let review = review(&f, "tuft:@v#1")?;
    let ran = trial_run(&f, &f.root.join("q.tuft"), Some(&review), &bin)?;
    let tuft = ran.tuft.as_ref().ok_or("tuft step")?;
    let row = tuft.projected.rows.first().ok_or("no row")?;
    let binding = &row.evidence.as_ref().ok_or("not retained")?.binding;
    assert!(retained(&Trial::open(&f.state)?, binding)?);
    let seen = fs::read_to_string(f.root.join("weft-bin/seen.weft"))?;
    assert!(seen.contains(&format!("asset call = citation \"{binding}\";")));
    assert!(!seen.contains("tuft:"), "{seen}");
    let weft = ran.weft.as_ref().ok_or("weft step")?;
    assert_eq!(weft.source_sha256, sha256_hex(seen.as_bytes()));
    assert_eq!(weft.bin_sha256, sha256_hex(STUB.as_bytes()));
    let printed = render(&ran);
    for expected in [
        "tuft @v retained=1 coverage=complete",
        &format!("\"tuft:@v#1\" = {binding} (src/lib.rs:2)"),
        "(results are hypothetical)",
        "  check bad: refuted",
        &format!("prepared edit {}", ran.prepared.prepared),
        &format!(
            "next: writfold trial approve {} --require ok",
            ran.prepared.run
        ),
    ] {
        assert!(printed.contains(expected), "{expected:?} not in\n{printed}");
    }
    let refused = approve(&f.state, &ran.prepared.run, &["bad".to_owned()]);
    assert!(refused.is_err_and(|e| e.to_string().contains("bad")));
    approve(&f.state, &ran.prepared.run, &["ok".to_owned()])?;
    Ok(())
}

#[test]
fn an_unresolved_placeholder_refuses_before_weft_runs() -> Result<()> {
    let f = fixture()?;
    let bin = f.root.join("weft-bin/weft");
    for citation in ["tuft:@v#2", "tuft:@other#1", "tuft:@v#0"] {
        let review = review(&f, citation)?;
        let refused = trial_run(&f, &f.root.join("q.tuft"), Some(&review), &bin);
        let message = refused.err().ok_or("accepted")?.to_string();
        assert!(
            message.starts_with("step weft refused: unresolved placeholder"),
            "{message}"
        );
    }
    assert!(!f.root.join("weft-bin/seen.weft").exists());
    assert!(no_worktrees(&f));
    Ok(())
}

#[test]
fn each_failure_names_its_step_and_leaves_no_worktree() -> Result<()> {
    let f = fixture()?;
    let bin = f.root.join("weft-bin/weft");
    let review = review(&f, "tuft:@v#1")?;
    let missing = f.root.join("missing.tuft");
    let failing = f.root.join("weft-bin/failing");
    executable(&failing, "#!/bin/sh\nexit 3\n")?;
    fs::write(f.root.join("edit.spl"), RECIPE.replace("run(1)", "absent"))?;
    let cases = [
        (missing.as_path(), bin.as_path(), "step tuft refused"),
        (
            &f.root.join("q.tuft"),
            failing.as_path(),
            "step weft refused",
        ),
        (
            &f.root.join("q.tuft"),
            &f.root.join("nope"),
            "step weft refused",
        ),
        (
            &f.root.join("q.tuft"),
            bin.as_path(),
            "step prepare refused",
        ),
    ];
    for (tuft, bin, expected) in cases {
        let refused = trial_run(&f, tuft, Some(&review), bin);
        let message = refused.err().ok_or("accepted")?.to_string();
        assert!(message.starts_with(expected), "{expected}: {message}");
    }
    assert!(no_worktrees(&f));
    let args: Vec<String> = [
        "run", "--repo", "r", "--file", "f", "--recipe", "e", "--weft", "w",
    ]
    .map(String::from)
    .into();
    let mut args = args;
    args.extend(["--state".to_owned(), f.state.display().to_string()]);
    let refused = crate::trial::cli::main(&args);
    assert!(refused.is_err_and(|e| e.to_string().contains("--weft-bin")));
    Ok(())
}
