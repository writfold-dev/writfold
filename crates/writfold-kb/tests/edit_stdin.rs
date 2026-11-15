//! `edit apply -`: one recipe read from stdin gives the same file bytes and
//! the same receipt, apart from the recipe's name (`stdin`), as the same
//! recipe from a file; a second `-` is a usage error; empty stdin is
//! refused with the recipe parser's own message.
mod support;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::edit::EditVerb;
use writfold_kb::edit::plan::recipes_from;
use writfold_kb::error::KbError;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const RETRY: &str =
    "edit \"src/retry.rs\" {\n  replace one text \"attempts = 5\" with \"attempts = 7\"\n}\n";
const CHILD: &str = "WRITFOLD_KB_EDIT_STDIN_CHILD";

fn request(kb: &Path, cwd: &Path, recipes: Vec<PathBuf>) -> writfold_kb::Request {
    let worktree = cwd.join("demo-service");
    writfold_kb::Request {
        verb: Verb::Edit(EditVerb::Apply {
            recipes,
            run: Some(RUN.into()),
            worktree: Some(worktree),
            dry_run: false,
        }),
        kb: Some(kb.to_path_buf()),
        cwd: cwd.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

/// The child: `edit apply -` with this process's stdin.
#[test]
fn child() -> TestResult {
    let (Ok(kb), Ok(cwd)) = (std::env::var(CHILD), std::env::var("WRITFOLD_KB_EDIT_CWD")) else {
        return Ok(());
    };
    let out = writfold_kb::run(&request(Path::new(&kb), Path::new(&cwd), vec!["-".into()]))?;
    print!("{}", out.text);
    Ok(())
}

/// The run's only edit receipt, with the world's paths made stable.
fn receipt(w: &World) -> TestResult<String> {
    let dir = w.kb.join(format!("runs/{RUN}/edits"));
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| Some(e.ok()?.path()))
        .collect();
    found.sort();
    let [one] = found.as_slice() else {
        return Err(format!("{} receipts", found.len()).into());
    };
    Ok(std::fs::read_to_string(one)?.replace(&w.base.display().to_string(), "<WORLD>"))
}

#[test]
fn a_recipe_on_stdin_writes_what_the_same_file_writes() -> TestResult {
    let by_file = world(&[])?;
    let file = by_file.base.join("retry.spl");
    std::fs::write(&file, RETRY)?;
    let out = writfold_kb::run(&request(&by_file.kb, &by_file.base, vec![file]))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    let piped = world(&[])?;
    let mut child = Command::new(std::env::current_exe()?)
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, &piped.kb)
        .env("WRITFOLD_KB_EDIT_CWD", &piped.base)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(RETRY.as_bytes())?;
    let out = child.wait_with_output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let edited = |w: &World| std::fs::read(w.code.join("src/retry.rs"));
    assert_eq!(edited(&piped)?, edited(&by_file)?);
    assert!(String::from_utf8_lossy(&edited(&piped)?).contains("attempts = 7"));
    let (a, b) = (receipt(&by_file)?, receipt(&piped)?);
    assert!(b.contains("stdin"), "{b}");
    assert_eq!(a.replace("retry.spl", "stdin"), b);
    Ok(())
}

#[test]
fn stdin_once_and_never_empty() -> TestResult {
    let cwd = std::env::temp_dir();
    let twice = recipes_from(&["-".into(), "-".into()], &cwd, RETRY.as_bytes());
    assert!(matches!(twice, Err(KbError::Usage(m)) if m.contains("may appear once")));
    let (one, paths) = recipes_from(&["-".into()], &cwd, RETRY.as_bytes())?;
    assert_eq!(
        (one[0].name.as_str(), paths[0].as_str()),
        ("stdin", "src/retry.rs")
    );
    let empty_file = cwd.join(format!("writfold-empty-{}.spl", std::process::id()));
    std::fs::write(&empty_file, "")?;
    let from_file = recipes_from(std::slice::from_ref(&empty_file), &cwd, &b""[..]);
    let _ = std::fs::remove_file(&empty_file);
    let from_stdin = recipes_from(&["-".into()], &cwd, &b""[..]);
    let name = format!("writfold-empty-{}.spl", std::process::id());
    match (from_file, from_stdin) {
        (Err(KbError::Refused(a)), Err(KbError::Refused(b))) => {
            assert_eq!(a.replace(&name, "stdin"), b);
        }
        _ => return Err("expected two refusals".into()),
    }
    let big = vec![b' '; 4 * 1024 * 1024 + 1];
    let large = recipes_from(&["-".into()], &cwd, big.as_slice());
    assert!(matches!(large, Err(KbError::Usage(m)) if m.contains("larger than")));
    Ok(())
}
