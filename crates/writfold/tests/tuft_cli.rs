//! The documented exit contract is exercised through the actual CLI binary.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn call(root: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_writfold"))
        .current_dir(root)
        .env_remove("WRITFOLD_STATE")
        .env_remove("WRITFOLD_KB")
        .env_remove("KCTX_CONFIG")
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root)
        .env("WRITFOLD_GIT", "/usr/bin/git")
        .args(args)
        .output()?)
}
fn git(root: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

#[test]
fn success_empty_partial_and_invalid_check_have_documented_exit_codes() -> Result<()> {
    let f = Fixture(std::env::temp_dir().join(format!(
        "writfold-tuft-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&f.0)?;
    fs::write(f.0.join("a.txt"), "needle needle\n")?;
    git(&f.0, &["init", "-q"])?;
    git(&f.0, &["add", "a.txt"])?;
    git(&f.0, &["commit", "-qm", "fixture"])?;
    let state = f.0.join("state");
    let state = state.to_str().ok_or("state path")?;
    let initialized = call(&f.0, &["trial", "init", "--state", state])?;
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    for (needle, limit, coverage, hits) in [
        ("needle", "100", "complete", 2),
        ("absent", "100", "complete", 0),
        ("needle", "1", "partial", 1),
    ] {
        let output = call(
            &f.0,
            &[
                "tuft", "grep", needle, "--state", state, "--json", "--limit", limit,
            ],
        )?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(body["find"]["coverage"]["state"], coverage);
        assert_eq!(body["find"]["hits"], hits);
    }
    fs::write(
        f.0.join("bad.tuft"),
        "@h := find text ;\n@v := project @h [unknown];\nemit @v;\n",
    )?;
    let output = call(&f.0, &["tuft", "check", "bad.tuft", "--json"])?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(body["ok"], false);
    assert!(
        body["diagnostics"]
            .as_array()
            .is_some_and(|ds| ds.len() >= 2)
    );
    let output = call(
        &f.0,
        &[
            "tuft",
            "grep",
            "needle",
            "--at",
            "missing-ref",
            "--state",
            state,
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let doc = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/tuft/language.md"),
    )?;
    assert!(doc.contains("| 0 | Successful query or check"));
    assert!(doc.contains("| 1 | Invalid arguments or program"));
    Ok(())
}
