//! `writfold tuft run` against a real repository and trial state: budget and
//! cursor, determinism, completeness, the hit limit, pinning and deferred
//! operators. Excerpts and citations are in `json_tests`.
use super::{Invocation, ProgramError, run};

use crate::trial::{Result, Trial};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tuft::emit::Format;
use writfold_store::backend::{Capability, Refusal};

static NEXT: AtomicU64 = AtomicU64::new(0);
const LIB: &str = "let a = 1;\nlet a = 1;\nrun(1);\n";
const NOTES: &str = "let a = café;\nnothing\n";

pub(super) struct Fixture {
    pub root: PathBuf,
    pub state: PathBuf,
    pub repo: PathBuf,
    pub commit: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
}

pub(super) fn fixture() -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-tuft-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::create_dir_all(repo.join("docs"))?;
    fs::write(repo.join("src/lib.rs"), LIB)?;
    fs::write(repo.join("docs/notes.md"), NOTES)?;
    for i in 0..5 {
        fs::write(repo.join(format!("src/m{i}.rs")), format!("let a = {i};\n"))?;
    }
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let commit = git(&repo, &["rev-parse", "HEAD"])?;
    let state = root.join("state");
    Trial::init(&state)?;
    Ok(Fixture {
        root,
        state,
        repo,
        commit,
    })
}

pub(super) fn tuft(
    f: &Fixture,
    program: &str,
    cursor: Option<&str>,
    format: Format,
) -> Result<String> {
    run(&Invocation {
        source: program.as_bytes(),
        name: "test.tuft",
        repo: &f.repo,
        state: &f.state,
        cursor,
        format,
    })
}

fn envelope<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output
        .split([' ', '\n'])
        .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
}

fn rows(output: &str) -> Vec<&str> {
    output
        .split_once("\ncolumns ")
        .map_or(Vec::new(), |(_, rest)| rest.lines().skip(1).collect())
}

pub(super) fn pipeline(f: &Fixture, budget: u64) -> String {
    format!(
        "@h := find text \"let a\" at \"{}\" limit 50;\n@e := retain @h;\n\
@v := project @e [path, line, binding, excerpt(max_chars: 6)];\nemit @v budget_tokens {budget};\n",
        f.commit
    )
}

#[test]
fn budget_overflow_reports_omissions_and_the_cursor_resumes() -> Result<()> {
    let f = fixture()?;
    let whole = tuft(&f, &pipeline(&f, 100_000), None, Format::Text)?;
    let all = rows(&whole);
    assert_eq!(all.len(), 8, "{whole}");
    assert_eq!(envelope(&whole, "cursor"), Some("none"));
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..all.len() {
        let page = tuft(&f, &pipeline(&f, 60), cursor.as_deref(), Format::Text)?;
        let shown = rows(&page);
        assert!(!shown.is_empty() && shown.len() < all.len(), "{page}");
        let omitted: usize = envelope(&page, "omitted").ok_or("omitted")?.parse()?;
        assert_eq!(seen.len() + shown.len() + omitted, all.len());
        seen.extend(shown.iter().map(|r| r.to_string()));
        match envelope(&page, "cursor") {
            Some("none") => break,
            Some(next) => cursor = Some(next.to_owned()),
            None => return Err("no cursor in envelope".into()),
        }
    }
    assert_eq!(seen, all);
    let other = pipeline(&f, 61);
    let refused = tuft(&f, &other, cursor.as_deref(), Format::Text);
    assert!(refused.is_err_and(|e| e.to_string().contains("different program")));
    Ok(())
}

#[test]
fn same_program_and_commit_give_byte_identical_output() -> Result<()> {
    let f = fixture()?;
    for format in [Format::Text, Format::Json] {
        let first = tuft(&f, &pipeline(&f, 120), None, format)?;
        let second = tuft(&f, &pipeline(&f, 120), None, format)?;
        assert_eq!(first, second);
    }
    Ok(())
}

#[test]
fn zero_results_over_a_complete_scope_are_reported_complete() -> Result<()> {
    let f = fixture()?;
    let program = format!(
        "@h := find text \"absent needle\" in \"src/\" at \"{}\";\n@v := project @h [path, line];\nemit @v;\n",
        f.commit
    );
    let output = tuft(&f, &program, None, Format::Text)?;
    assert!(
        output.contains("coverage=complete files=6/6 hits=0"),
        "{output}"
    );
    assert!(rows(&output).is_empty());
    Ok(())
}

#[test]
fn the_hit_limit_stops_the_search_explicitly() -> Result<()> {
    let f = fixture()?;
    for (matcher, label) in [("text \"let a\"", "text"), ("rx\"let [a-z]\"", "rx")] {
        let program = format!(
            "@h := find {matcher} at \"{}\" limit 3;\n@v := project @h [path, range];\nemit @v;\n",
            f.commit
        );
        let output = tuft(&f, &program, None, Format::Text)?;
        assert!(
            output.contains("coverage=partial files=2/7 hits=3 stopped=hit-limit"),
            "{label}: {output}"
        );
        assert_eq!(rows(&output).len(), 3);
    }
    Ok(())
}

#[test]
fn only_a_full_existing_commit_is_accepted() -> Result<()> {
    let f = fixture()?;
    for pin in ["HEAD", "main", &f.commit[..12]] {
        let program = pipeline(&f, 100).replace(&f.commit, pin);
        assert!(tuft(&f, &program, None, Format::Text).is_err(), "{pin}");
    }
    let tree = git(&f.repo, &["rev-parse", "HEAD^{tree}"])?;
    for pin in ["0".repeat(40), tree] {
        let program = pipeline(&f, 100).replace(&f.commit, &pin);
        let refused = tuft(&f, &program, None, Format::Text)
            .err()
            .ok_or("accepted")?;
        let refusal = refused.downcast_ref::<Refusal>().ok_or("untyped refusal")?;
        assert!(matches!(refusal, Refusal::Pin(_)), "{refusal}");
    }
    Ok(())
}

#[test]
fn deferred_operators_refuse_as_unsupported_capabilities() -> Result<()> {
    let f = fixture()?;
    let program = format!(
        "@h := find similar \"x\" at \"{}\";\n@v := project @h [path];\nemit @v;\n",
        f.commit
    );
    let refused = tuft(&f, &program, None, Format::Text)
        .err()
        .ok_or("accepted")?;
    let program_error = refused.downcast_ref::<ProgramError>().ok_or("untyped")?;
    assert_eq!(
        program_error.refusals(),
        [Refusal::Unsupported(Capability::Semantic)]
    );
    assert!(refused.to_string().contains("error[T301]"));
    Ok(())
}
