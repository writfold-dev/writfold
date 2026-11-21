//! Graph answers read a bounded number of edges (#60): around a hub (a
//! project every decision names), `impact`, `uses` and `context` under a
//! small `WRITFOLD_KB_WORK` show what they reached, say they stopped, and
//! never claim `coverage=complete`; with room to read everything they answer
//! as before.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Kb(PathBuf);

impl Drop for Kb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
    if !ok {
        return Err(format!("git {args:?}").into());
    }
    Ok(())
}

/// `n` decisions in project `harbor`, each mentioning the hub decision 0.
fn hub(n: usize) -> Result<Kb> {
    let dir = std::env::temp_dir().join(format!("writfold-work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let kb = Kb(dir.clone());
    std::fs::create_dir_all(dir.join("inventory/assets"))?;
    std::fs::create_dir_all(dir.join("proposals/decision"))?;
    std::fs::create_dir_all(dir.join("harbor"))?;
    std::fs::write(dir.join("harbor/README.md"), "# Harbor\n")?;
    std::fs::write(
        dir.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    for i in 0..n {
        std::fs::write(
            dir.join(format!("proposals/decision/2026-01-01-d{i:03}.md")),
            format!(
                "---\nid: decision/2026-01-01-d{i:03}\nkind: decision\ntitle: D{i}\nstatus: accepted\ncreated: 2026-01-01\nupdated: 2026-01-01\nproject: harbor\n---\n\n## Context\n\nSee decision/2026-01-01-d000.\n"
            ),
        )?;
    }
    git(&dir, &["init", "-q"])?;
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-q", "-m", "kb"])?;
    Ok(kb)
}

fn kb(dir: &Path, args: &[&str], work: &str) -> Result<(String, i32)> {
    let out = Command::new(env!("CARGO_BIN_EXE_writfold"))
        .arg("kb")
        .args(args)
        .current_dir(dir)
        .env("WRITFOLD_KB_WORK", work)
        .env_remove("WRITFOLD_KB")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

#[test]
fn a_hub_is_read_up_to_the_work_limit_and_never_called_complete() -> Result {
    let k = hub(40)?;
    let cases: [&[&str]; 3] = [
        &[
            "impact",
            "project:harbor",
            "--direction",
            "in",
            "--depth",
            "1",
        ],
        &["uses", "decision/2026-01-01-d000"],
        &["context", "project:harbor"],
    ];
    for args in cases {
        let (text, _) = kb(&k.0, args, "1000000")?;
        assert!(
            text.contains("coverage=complete"),
            "{args:?} unbounded: {text}"
        );
        assert!(!text.contains("work limit"), "{args:?}: {text}");
        let (text, exit_small) = kb(&k.0, args, "5")?;
        assert!(
            text.contains("coverage=partial"),
            "{args:?} limited: {text}"
        );
        assert!(text.contains("work limit of 5"), "{args:?}: {text}");
        assert_eq!(exit_small, 2, "{args:?}: {text}");
    }
    let (text, _) = kb(&k.0, cases[0], "5")?;
    assert!(text.contains("examined=5"), "{text}");
    let rows = text.lines().filter(|l| l.starts_with("decision ")).count();
    assert!((1..=5).contains(&rows), "a page, not the degree: {text}");
    Ok(())
}
