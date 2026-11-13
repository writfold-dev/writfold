use super::data::{Many, Output};
use super::{args, fixture, git, run};
use crate::trial::Result;
use crate::tuft::tests::Fixture;
use std::{fs, path::PathBuf};
use writfold_kb::write::{
    Write,
    run::{Checkpoint, NewRun},
};
use writfold_kb::{Request, Verb};

fn kb(f: &Fixture) -> Result<PathBuf> {
    let kb = f.root.join("kb");
    fs::create_dir_all(kb.join("inventory/assets"))?;
    fs::write(kb.join("README.md"), "# Test notes\n")?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-qm", "notes"])?;
    let request = Request {
        verb: Verb::Write(Write::Checkpoint(Checkpoint {
            run: None,
            new_run: Some(NewRun {
                project: "notes".into(),
                task: "retrieval".into(),
                mode: None,
            }),
            note: "Start".into(),
        })),
        kb: Some(kb.clone()),
        cwd: f.repo.clone(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(1_794_355_200),
        probe: writfold_kb::write::no_probe,
    };
    writfold_kb::run(&request)?;
    Ok(kb)
}

#[test]
fn mapped_repositories_keep_distinct_pins_citations_and_receipts() -> Result<()> {
    let f = fixture()?;
    let kb = kb(&f)?;
    let other = f.root.join("other");
    git(
        &f.root,
        &[
            "clone",
            "-q",
            &f.repo.to_string_lossy(),
            &other.to_string_lossy(),
        ],
    )?;
    fs::write(other.join("src/lib.rs"), "run(1);\nrun(1);\n")?;
    git(&other, &["commit", "-qam", "other"])?;
    let other_pin = git(&other, &["rev-parse", "HEAD"])?;
    fs::write(
        kb.join(".git/writfold/repos"),
        format!("alpha {}\nbeta {}\n", f.repo.display(), other.display()),
    )?;
    let mut query = args(
        &f,
        &[
            "--kb",
            &kb.to_string_lossy(),
            "--json",
            "--cite",
            "--repo",
            "beta@HEAD",
        ],
    );
    query[3] = "alpha@HEAD".into();
    let (text, notes) = run(&f, &query)?;
    let body: Many = serde_json::from_str(&text)?;
    let repos = &body.repositories;
    assert_eq!(repos[0].repo, "alpha");
    assert_eq!(repos[1].repo, "beta");
    assert_eq!(repos[0].result.commit, f.commit);
    assert_eq!(repos[1].result.commit, other_pin);
    for (index, name) in ["alpha", "beta"].iter().enumerate() {
        let token = &repos[index].result.cites[0];
        assert!(token.contains(&format!("repo={name} ")), "{token}");
    }
    assert_eq!(body.overall.coverage, "complete");
    assert_eq!(body.overall.hits, 3);
    assert_eq!(
        notes.iter().filter(|n| n.starts_with("recorded ")).count(),
        2
    );
    let run = fs::read_dir(kb.join("runs"))?.next().ok_or("run")??.path();
    for entry in fs::read_dir(run.join("retrievals"))? {
        let receipt = fs::read_to_string(entry?.path())?;
        assert!(
            !receipt.contains("HEAD"),
            "receipt must store pins, not input refs: {receipt}"
        );
        assert!(receipt.contains(&f.commit) || receipt.contains(&other_pin));
    }
    Ok(())
}

#[test]
fn dirty_untracked_citations_never_claim_a_git_blob_or_freshness() -> Result<()> {
    let f = fixture()?;
    let kb = kb(&f)?;
    fs::write(
        kb.join(".git/writfold/repos"),
        format!("repo {}\n", f.repo.display()),
    )?;
    fs::write(f.repo.join("src/new.rs"), "run(1); // uncommitted\n")?;
    let query = args(
        &f,
        &[
            "--kb",
            &kb.to_string_lossy(),
            "--json",
            "--cite",
            "--working-tree",
            "--in",
            "src/new.rs",
        ],
    );
    let (text, _) = run(&f, &query)?;
    let body: Output = serde_json::from_str(&text)?;
    let token = &body.cites[0];
    assert!(token.contains(" dirty="), "{token}");
    assert!(token.contains("blob=dirty:sha256:"), "{token}");
    assert!(
        body.rows[0]
            .excerpt
            .as_ref()
            .is_some_and(|e| e.text.contains("uncommitted"))
    );
    fs::write(
        kb.join("inventory/assets/dirty.md"),
        format!("# Dirty evidence\n{token}\n"),
    )?;
    let request = Request {
        verb: Verb::Stale {
            scope: None,
            offline: false,
            cached: false,
        },
        kb: Some(kb.clone()),
        cwd: kb,
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(1_794_355_200),
        probe: writfold_kb::write::no_probe,
    };
    let stale = writfold_kb::run(&request)?.text;
    assert!(
        stale.contains("unverifiable") && stale.contains("dirty working-tree"),
        "{stale}"
    );
    Ok(())
}
