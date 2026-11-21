//! Record options belong to the kind that consumes them (#85): through the
//! real binary, another kind's option is a usage error (exit 1) that
//! leaves the KB and both proposal queues untouched, before or after the
//! kind, for `record` and for `propose --local|--tracked`; valid calls keep
//! every field; each kind's help shows only its own options.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");

struct Kb(PathBuf);

impl Drop for Kb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn kb(name: &str) -> Result<Kb> {
    let dir = std::env::temp_dir().join(format!("writfold-options-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("inventory/assets"))?;
    std::fs::create_dir_all(dir.join("harbor"))?;
    std::fs::write(dir.join("harbor/README.md"), "# Harbor\n")?;
    std::fs::write(
        dir.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    git(&dir, &["init", "-q", "-b", "main"])?;
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-q", "-m", "kb"])?;
    Ok(Kb(std::fs::canonicalize(dir)?))
}

fn kb_run(k: &Kb, args: &[&str]) -> Result<(String, i32)> {
    let out = Command::new(WRITFOLD)
        .arg("kb")
        .args(args)
        .current_dir(&k.0)
        .env("WRITFOLD_KB", &k.0)
        .env("WRITFOLD_SESSION", "s1")
        .env("HOME", k.0.join(".home"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

/// Everything a write or a proposal could change: the work tree's status
/// and the local queue under `.git/`.
fn state(k: &Kb) -> Result<String> {
    let mut seen = git(&k.0, &["status", "--porcelain", "--untracked-files=all"])?;
    let mut stack = vec![k.0.join(".git/writfold")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if !path.to_string_lossy().contains("kb-index") {
                seen.push_str(&format!("{}\n", path.display()));
            }
        }
    }
    Ok(seen)
}

#[test]
fn another_kinds_option_is_refused_before_anything_is_written() -> Result {
    let k = kb("wrong")?;
    let before = state(&k)?;
    let wrong: [&[&str]; 9] = [
        &[
            "record",
            "investigation",
            "--title",
            "Temperature again",
            "--prs",
            "PR-42",
        ],
        &[
            "record",
            "--prs",
            "PR-42",
            "investigation",
            "--title",
            "before the kind",
        ],
        &[
            "record",
            "ticket",
            "AB-1",
            "--title",
            "t",
            "--question",
            "why?",
        ],
        &[
            "record",
            "ticket",
            "AB-1",
            "--title",
            "t",
            "--project",
            "harbor",
        ],
        &[
            "record",
            "decision",
            "--title",
            "t",
            "--target-source",
            "s",
            "--prs",
            "p",
        ],
        &["record", "link", "relates-to", "a", "b", "--title", "t"],
        &[
            "propose",
            "investigation",
            "--title",
            "t",
            "--prs",
            "PR-42",
            "--local",
        ],
        &[
            "propose",
            "investigation",
            "--title",
            "t",
            "--prs",
            "PR-42",
            "--tracked",
        ],
        &[
            "propose",
            "ticket",
            "AB-1",
            "--title",
            "t",
            "--trigger",
            "x",
        ],
    ];
    for args in wrong {
        let (text, exit) = kb_run(&k, args)?;
        assert_eq!(exit, 1, "{args:?}: {text}");
        assert!(text.contains("does not take --"), "{args:?}: {text}");
        assert_eq!(state(&k)?, before, "{args:?} changed the KB or a queue");
    }
    Ok(())
}

#[test]
fn valid_calls_keep_every_field_direct_and_proposed() -> Result {
    let k = kb("valid")?;
    let (text, exit) = kb_run(
        &k,
        &[
            "record", "ticket", "AB-1", "--title", "Stall", "--assets", "demo", "--prs", "PR-7",
        ],
    )?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = kb_run(
        &k,
        &[
            "record",
            "--project",
            "harbor",
            "decision",
            "--title",
            "Pick",
            "--target-source",
            "harbor/README.md",
        ],
    )?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = kb_run(
        &k,
        &[
            "propose",
            "investigation",
            "--title",
            "Temperature again",
            "--signatures",
            "sig-a",
            "--question",
            "why?",
            "--local",
        ],
    )?;
    assert_eq!(exit, 0, "{text}");
    let written = git(&k.0, &["status", "--porcelain", "--untracked-files=all"])?;
    let ticket = written
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .last()
                .filter(|p| p.contains("AB-1") || p.contains("ab-1"))
        })
        .ok_or_else(|| format!("no ticket written: {written}"))?;
    assert!(
        std::fs::read_to_string(k.0.join(ticket))?.contains("PR-7"),
        "the ticket keeps --prs"
    );
    let decision = written
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .last()
                .filter(|p| p.contains("decision") && p.ends_with("proposal.md"))
        })
        .ok_or_else(|| format!("no decision written: {written}"))?;
    assert!(
        std::fs::read_to_string(k.0.join(decision))?.contains("harbor"),
        "the decision keeps --project"
    );
    let (shown, exit) = kb_run(&k, &["proposals", "list"])?;
    assert_eq!(exit, 0, "{shown}");
    assert!(shown.contains("Temperature again"), "{shown}");
    Ok(())
}

#[test]
fn each_kinds_help_shows_only_its_own_options() -> Result {
    let k = kb("help")?;
    let flags = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| l.trim_start().strip_prefix("--"))
            .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
            .collect()
    };
    let (help, exit) = kb_run(&k, &["record", "investigation", "--help"])?;
    assert_eq!(exit, 0, "{help}");
    let shown = flags(&help);
    assert!(
        shown.iter().any(|f| f == "question") && !shown.iter().any(|f| f == "prs"),
        "{help}"
    );
    assert!(
        !shown.iter().any(|f| f == "tracked"),
        "record has no store: {help}"
    );
    let (help, _) = kb_run(&k, &["propose", "ticket", "--help"])?;
    let shown = flags(&help);
    for want in ["prs", "tracked", "local", "session", "run"] {
        assert!(shown.iter().any(|f| f == want), "--{want}: {help}");
    }
    assert!(!shown.iter().any(|f| f == "question"), "{help}");
    let (help, _) = kb_run(&k, &["record", "decision", "--help"])?;
    assert!(flags(&help).iter().any(|f| f == "project"), "{help}");
    Ok(())
}
