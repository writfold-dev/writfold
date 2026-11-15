//! Kill tests of `kb stale --record` (design section 38): the child re-runs
//! this test binary, records with a fake checker on `PATH` and aborts at
//! each step of the write protocol. After every death the kind's receipt
//! directory holds no receipt or one complete, strictly readable receipt;
//! a leftover temporary is dot-named and never read; recording again
//! succeeds.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use writfold_kb::stale::checker::receipt;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHILD: &str = "WRITFOLD_KB_RECEIPT_CHILD";
const NOW: i64 = 1_794_355_200;
const KIND: &str = "schema = \"writfold-source/1\"\nkind = \"src\"\nfreshness = \"checked\"\nchecker = \"fault-src\"\nfields = [\"path\"]\nmax_age = \"7d\"\n";
const FAKE: &str = "#!/bin/sh\nwhile IFS= read -r line; do\n  id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n  *describe*) echo '{\"v\":1,\"name\":\"fault-src\",\"version\":\"0.1\",\"kinds\":[\"src\"],\"ops\":[\"check\"],\"token_versions\":[1]}' ;;\n  *) echo \"{\\\"v\\\":1,\\\"id\\\":\\\"$id\\\",\\\"status\\\":\\\"fresh\\\"}\" ;;\n  esac\ndone\n";

fn git(dir: &Path, args: &[&str]) -> TestResult {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .status()?
        .success();
    if !ok {
        return Err(format!("git {args:?} failed").into());
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

/// A KB with two `src` tokens and the fake checker in `<dir>/bin`.
fn world(name: &str) -> TestResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!("writfold-receipt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let kb = dir.join("kb");
    put(
        &kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    put(&kb.join(".writfold/sources/src.toml"), KIND)?;
    put(
        &kb.join("notes/sources.md"),
        "# Sources\n\n⟦cite kind=src path=a.c⟧\n\n⟦cite kind=src path=b.c⟧\n",
    )?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "kb"])?;
    let fake = dir.join("bin/writfold-check-fault-src");
    put(&fake, FAKE)?;
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))?;
    Ok(dir)
}

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

fn request(kb: &Path) -> Request {
    Request {
        verb: Verb::StaleRecord { scope: None },
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: abort_at,
    }
}

/// The child: one recording that aborts at the step in the environment.
#[test]
fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    writfold_kb::run(&request(Path::new(&kb)))?;
    Ok(())
}

/// The receipts in the kind's directory: each one parses whole.
fn receipts(kb: &Path) -> TestResult<usize> {
    let dir = kb.join(receipt::DIR).join("src");
    let mut found = 0;
    for entry in std::fs::read_dir(&dir).into_iter().flatten() {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let text = std::fs::read_to_string(dir.join(&name))?;
        let path = format!("{}/src/{name}", receipt::DIR);
        let parsed = receipt::parse(&path, "src", &text).map_err(|e| format!("{e}:\n{text}"))?;
        assert_eq!(parsed.verdicts.len(), 2, "{text}");
        found += 1;
    }
    Ok(found)
}

#[test]
fn a_kill_at_each_step_leaves_no_receipt_or_a_whole_one() -> TestResult {
    let mut published = 0;
    for step in Step::ALL {
        let dir = world(step.name())?;
        let kb = dir.join("kb");
        let path = format!(
            "{}:{}",
            dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let status = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &kb)
            .env("WRITFOLD_KB_FAULT_STEP", step.name())
            .env("PATH", &path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(
            !status.success(),
            "the child did not stop at {}",
            step.name()
        );
        let found = receipts(&kb)?;
        assert!(found <= 1, "{}: {found} receipts", step.name());
        published += found;
        // Recording again, with nothing stopping it, completes.
        let again = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &kb)
            .env("PATH", &path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(again.success(), "{}", step.name());
        assert_eq!(
            receipts(&kb)?,
            1,
            "{}: the later receipt supersedes",
            step.name()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(published > 0, "no step published a receipt before dying");
    Ok(())
}
