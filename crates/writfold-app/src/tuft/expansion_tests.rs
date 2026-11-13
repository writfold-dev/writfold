//! Observable pin, work-tree and glob behavior at the real Git boundary.
use super::tests::{Fixture, fixture, git};
use super::{Executed, Invocation, execute};
use crate::trial::Result;
use std::fs;
use std::path::Path;
use tuft::emit::{Cursor, Format};

pub(super) fn program(f: &Fixture, matcher: &str, scope: &str) -> String {
    format!(
        "@h := find {matcher} in \"{scope}\" at \"{}\"; @e := retain @h around lines 0; @v := project @e [path, line, binding, excerpt(max_chars: 80)]; emit @v budget_tokens 10000;",
        f.commit
    )
}

pub(super) fn run_at(
    f: &Fixture,
    repo: &Path,
    source: &str,
    working_tree: bool,
    index: Option<&Path>,
) -> Result<Executed> {
    execute(&Invocation {
        source: source.as_bytes(),
        name: "expansion.tuft",
        repo,
        state: &f.state,
        cursor: None,
        format: Format::Json,
        working_tree,
        index,
    })
}

#[test]
fn linked_worktree_and_submodule_keep_pinned_evidence() -> Result<()> {
    let f = fixture()?;
    let source = program(&f, "text \"let a\"", "src/lib.rs");
    let expected = run_at(&f, &f.repo, &source, false, None)?.emitted.printed();
    let linked = f.root.join("linked");
    let linked_text = linked.to_str().ok_or("non-UTF-8 fixture")?;
    git(
        &f.repo,
        &["worktree", "add", "--detach", linked_text, &f.commit],
    )?;
    assert!(linked.join(".git").is_file());
    assert_eq!(
        run_at(&f, &linked, &source, false, None)?.emitted.printed(),
        expected
    );
    let parent = f.root.join("parent");
    fs::create_dir(&parent)?;
    git(&parent, &["init", "-q"])?;
    git(
        &parent,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            f.repo.to_str().ok_or("fixture path")?,
            "module",
        ],
    )?;
    let module = parent.join("module");
    assert!(module.join(".git").is_file());
    assert_eq!(
        run_at(&f, &module, &source, false, None)?.emitted.printed(),
        expected
    );
    Ok(())
}

#[test]
fn changed_and_untracked_bytes_are_never_pinned() -> Result<()> {
    let f = fixture()?;
    fs::write(f.repo.join("src/lib.rs"), "let a = dirty;\n")?;
    fs::write(f.repo.join("new.rs"), "let a = untracked;\n")?;
    fs::write(f.repo.join("ignored.rs"), "let a = ignored;\n")?;
    fs::write(f.repo.join(".gitignore"), "ignored.rs\n")?;
    let source = program(&f, "text \"let a\"", "");
    let result = run_at(&f, &f.repo, &source, true, None)?;
    assert_eq!(result.dirty_files.len(), 2);
    assert_eq!(
        result.dirty_files.get("src/lib.rs").map(Vec::as_slice),
        Some(b"let a = dirty;\n".as_slice())
    );
    assert!(result.emitted.printed().contains("dirty:sha256:"));
    assert!(!result.projected.rows.iter().any(|r| r.path == "ignored.rs"));
    for row in &result.projected.rows {
        let evidence = row.evidence.as_ref().ok_or("missing evidence")?;
        assert_eq!(
            evidence.binding.starts_with("dirty:"),
            result.dirty_files.contains_key(&row.path)
        );
        if evidence.binding.starts_with("dirty:") {
            assert!(
                !f.state
                    .join("evidence")
                    .join(format!("{}.json", evidence.binding))
                    .exists()
            );
        }
    }
    let frozen = run_at(&f, &f.repo, &source, false, None)?;
    assert!(frozen.dirty_files.is_empty());
    assert!(!frozen.emitted.printed().contains("untracked"));
    Ok(())
}

#[test]
fn missing_and_symlink_files_make_working_coverage_partial() -> Result<()> {
    let f = fixture()?;
    fs::remove_file(f.repo.join("src/m0.rs"))?;
    let outside = f.root.join("outside");
    fs::write(&outside, "SECRET\n")?;
    std::os::unix::fs::symlink(&outside, f.repo.join("link.rs"))?;
    let source = program(&f, "text \"SECRET\"", "");
    let result = run_at(&f, &f.repo, &source, true, None)?;
    assert!(result.projected.rows.is_empty());
    assert!(result.emitted.trailer.contains("coverage=partial"));
    assert!(result.emitted.trailer.contains("skipped=2"));
    #[derive(serde::Deserialize)]
    struct Counts {
        skipped_files: usize,
    }
    #[derive(serde::Deserialize)]
    struct Output {
        working_tree: Counts,
    }
    let value: Output = serde_json::from_str(&result.emitted.view)?;
    assert_eq!(value.working_tree.skipped_files, 2);
    Ok(())
}

#[test]
fn glob_counts_only_selected_paths_even_when_no_matches() -> Result<()> {
    let f = fixture()?;
    let source = program(&f, "text \"absent\"", "src/**/m?.rs");
    for working in [false, true] {
        let result = run_at(&f, &f.repo, &source, working, None)?;
        assert!(
            result
                .emitted
                .trailer
                .contains("coverage=complete files=5/5 hits=0")
        );
    }
    Ok(())
}

#[test]
fn working_cursor_binds_bytes_without_projected_excerpts() -> Result<()> {
    let f = fixture()?;
    let source = format!(
        "@h := find text \"let a\" at \"{}\"; @v := project @h [path, line]; emit @v budget_tokens 8;",
        f.commit
    );
    let first = run_at(&f, &f.repo, &source, true, None)?;
    #[derive(serde::Deserialize)]
    struct Output {
        cursor: Option<String>,
    }
    let value: Output = serde_json::from_str(&first.emitted.view)?;
    let cursor = Cursor::parse(value.cursor.as_deref().ok_or("expected cursor")?)?;
    fs::write(f.repo.join("src/m0.rs"), "let a = changed;\n")?;
    let result = execute(&Invocation {
        source: source.as_bytes(),
        name: "cursor.tuft",
        repo: &f.repo,
        state: &f.state,
        cursor: Some(&cursor),
        format: Format::Json,
        working_tree: true,
        index: None,
    });
    assert!(result.is_err());
    Ok(())
}
