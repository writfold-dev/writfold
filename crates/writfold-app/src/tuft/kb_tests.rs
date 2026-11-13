//! The KB adapter: default output unchanged, `--cite` tokens that check
//! fresh under `writfold kb stale` and then move or change after a code
//! edit, and (in `receipts`) retrieval receipts in the current run.
mod receipts;
mod state;

use super::cli::{Context, run_command};
use super::tests::{Fixture, fixture, git, pipeline, tuft};
use crate::trial::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tuft::emit::Format;
use writfold_kb::{Request, Verb};

const NOW: i64 = 1_794_355_200;

/// `--json --cite`: the rows and the tokens beside them.
#[derive(serde::Deserialize)]
struct Cited {
    rows: Vec<serde::de::IgnoredAny>,
    cites: Vec<String>,
}

/// A KB beside the fixture repository, mapping it as `repo`.
fn kb(f: &Fixture) -> Result<PathBuf> {
    let kb = f.root.join("kb");
    fs::create_dir_all(kb.join("inventory/assets"))?;
    fs::write(kb.join("README.md"), "# Notes\n")?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-qm", "kb"])?;
    let map = format!("repo {}\n", f.repo.display());
    fs::create_dir_all(kb.join(".git/writfold"))?;
    fs::write(kb.join(".git/writfold/repos"), map)?;
    Ok(kb)
}

fn kb_run(kb: &Path, verb: Verb) -> Result<String> {
    let request = Request {
        verb,
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    };
    Ok(writfold_kb::run(&request)?.text)
}

/// `tuft run` through the CLI path with `extra` flags.
fn cli(f: &Fixture, program: &str, extra: &[&str], cwd: &Path) -> Result<(String, Vec<String>)> {
    let file = f.root.join(format!("p{}.tuft", extra.len()));
    let _ = fs::remove_file(&file);
    fs::write(&file, program)?;
    let paths = [&file, &f.repo, &f.state].map(|p| p.to_string_lossy().into_owned());
    let mut args = vec![
        "run".into(),
        paths[0].clone(),
        "--repo".into(),
        paths[1].clone(),
    ];
    args.extend(["--state".into(), paths[2].clone()]);
    args.extend(extra.iter().map(|s| s.to_string()));
    // Discovery only from inside a test KB, so no test can find the
    // operator's own KB ($WRITFOLD_KB or the prototype's configured root).
    let context = Context {
        cwd: cwd.to_path_buf(),
        discover: cwd.join("inventory/assets").is_dir()
            && std::env::var_os("WRITFOLD_KB").is_none(),
        now: Some(NOW),
    };
    run_command(&args, context)
}

/// `tuft run` of a program with one hit, `src/lib.rs:3: run(1);`.
fn one(f: &Fixture, extra: &[&str], cwd: &Path) -> Result<(String, Vec<String>)> {
    let program = format!(
        "@h := find text \"run(1)\" in \"src/\" at \"{}\";\n@e := retain @h around lines 0;\n\
@v := project @e [path, line, excerpt(max_chars: 40)];\nemit @v;\n",
        f.commit
    );
    cli(f, &program, extra, cwd)
}

#[test]
fn default_output_is_unchanged_and_cites_align_with_shown_rows() -> Result<()> {
    let f = fixture()?;
    for format in [Format::Compact, Format::Verbose, Format::Json] {
        let flag = match format {
            Format::Compact => vec![],
            Format::Verbose => vec!["--verbose"],
            Format::Json => vec!["--json"],
        };
        let plain = tuft(&f, &pipeline(&f, 60), None, format)?;
        let (shown, notes) = cli(&f, &pipeline(&f, 60), &flag, &f.root)?;
        assert_eq!((shown.as_str(), notes.len()), (plain.as_str(), 0));
        let mut cited = flag.clone();
        cited.push("--cite");
        let (with, _) = cli(&f, &pipeline(&f, 60), &cited, &f.root)?;
        if format == Format::Json {
            let parsed: Cited = serde_json::from_str(&with)?;
            assert_eq!(parsed.cites.len(), parsed.rows.len());
            assert!(
                parsed
                    .cites
                    .iter()
                    .all(|t| t.starts_with("\u{27e6}cite kind=code "))
            );
            continue;
        }
        let lines: Vec<&str> = with.split('\n').collect();
        let kept: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| !l.starts_with("  cite: "))
            .collect();
        assert_eq!(kept.join("\n"), plain);
        for (i, line) in lines.iter().enumerate() {
            if let Some(token) = line.strip_prefix("  cite: ") {
                let row = lines[i - 1];
                let path = row.split([':', ' ']).next().ok_or("row path")?;
                assert!(token.contains(&format!(" path={path} ")), "{row} / {token}");
            }
        }
        assert!(lines.iter().any(|l| l.starts_with("  cite: ")), "{with}");
    }
    Ok(())
}

#[test]
fn cited_rows_are_fresh_then_moved_then_changed() -> Result<()> {
    let f = fixture()?;
    let kb = kb(&f)?;
    let (shown, notes) = one(&f, &["--cite"], &f.root)?;
    let mut lines = shown.lines();
    assert_eq!(lines.next(), Some("src/lib.rs:3: run(1);"));
    let token = lines
        .next()
        .and_then(|l| l.strip_prefix("  cite: "))
        .ok_or("no cite line")?;
    let rev = format!(" rev={} path=src/lib.rs L=3-3 blob=", f.commit);
    assert!(token.starts_with("\u{27e6}cite kind=code repo=repo") && token.contains(&rev));
    assert!(
        lines
            .next()
            .is_some_and(|l| l.starts_with("# coverage=complete"))
    );
    // Without a KB the name is unmapped, and the note says so.
    assert!(
        notes.iter().any(|n| n.contains("repo=repo is not in")),
        "{notes:?}"
    );
    fs::create_dir_all(kb.join("notes"))?;
    fs::write(
        kb.join("notes/runner.md"),
        format!("# Notes\n\nThe runner {token}.\n"),
    )?;
    let scope = Some("notes/runner.md".to_owned());
    let stale = |kb: &Path| {
        kb_run(
            kb,
            Verb::Stale {
                scope: scope.clone(),
                offline: false,
            },
        )
    };
    let fresh = stale(&kb)?;
    assert!(
        fresh.contains(" checked=1 ") && !fresh.contains("runner.md:"),
        "{fresh}"
    );
    fs::write(
        f.repo.join("src/lib.rs"),
        format!(
            "// a\n// b\n{}",
            fs::read_to_string(f.repo.join("src/lib.rs"))?
        ),
    )?;
    git(&f.repo, &["commit", "-qam", "shift"])?;
    let moved = stale(&kb)?;
    assert!(moved.contains("!moved") && moved.contains("5-5"), "{moved}");
    fs::write(
        f.repo.join("src/lib.rs"),
        "// a\n// b\nlet a = 1;\nlet a = 1;\nrun(2);\n",
    )?;
    git(&f.repo, &["commit", "-qam", "edit"])?;
    let changed = stale(&kb)?;
    assert!(changed.contains("!changed"), "{changed}");
    Ok(())
}
