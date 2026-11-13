use super::{Context, outcome, run_command};
use crate::trial::Result;
use crate::tuft::tests::{Fixture, fixture, git, pipeline};
use data::{Check, Output};
use std::fs;

mod data;
mod json;
mod repositories;

fn args(f: &Fixture, extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "grep".into(),
        "run(1)".into(),
        "--repo".into(),
        f.repo.to_string_lossy().into_owned(),
        "--state".into(),
        f.state.to_string_lossy().into_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    args
}

fn run(f: &Fixture, args: &[String]) -> Result<(String, Vec<String>)> {
    run_command(
        args,
        Context {
            cwd: f.root.clone(),
            discover: false,
            now: Some(1_794_355_200),
        },
    )
}

#[test]
fn grep_pins_refs_and_prints_the_exact_program_used() -> Result<()> {
    let f = fixture()?;
    git(&f.repo, &["branch", "topic"])?;
    git(&f.repo, &["tag", "release"])?;
    for reference in ["HEAD", "topic", "release", &f.commit[..12]] {
        let (text, notes) = run(
            &f,
            &args(&f, &["--at", reference, "--json", "--print-program"]),
        )?;
        let body: Output = serde_json::from_str(&text)?;
        assert_eq!(body.commit, f.commit);
        let at = body.at.ok_or("resolved ref")?;
        assert_eq!(at.requested, reference);
        assert_eq!(at.commit, f.commit);
        assert_eq!(body.rows[0].path, "src/lib.rs");
        let source = notes
            .last()
            .and_then(|n| n.split_once('\n'))
            .ok_or("no program")?
            .1;
        let plan = tuft::compile(source.as_bytes()).map_err(|ds| format!("{ds:?}"))?;
        assert_eq!(body.program, format!("sha256:{}", plan.program));
        assert_eq!(plan.find.commit, f.commit);
    }
    Ok(())
}

#[test]
fn saved_files_and_pinned_mode_never_accept_movable_refs() -> Result<()> {
    let f = fixture()?;
    assert!(run(&f, &args(&f, &["--pinned"])).is_err());
    let file = f.root.join("query.tuft");
    fs::write(&file, pipeline(&f, 4000).replace(&f.commit, "HEAD"))?;
    let mut saved = args(&f, &[]);
    saved[0] = "run".into();
    saved[1] = file.to_string_lossy().into_owned();
    assert!(run(&f, &saved).is_err_and(|e| e.to_string().contains("T206")));
    fs::write(&file, pipeline(&f, 4000))?;
    saved.extend(["--at".into(), "HEAD".into()]);
    assert!(run(&f, &saved).is_err());
    Ok(())
}

#[test]
fn glob_scope_does_not_search_sibling_extensions() -> Result<()> {
    let f = fixture()?;
    let mut query = args(&f, &["--in", "src/*.rs", "--json"]);
    query[1] = "let a".into();
    let (text, _) = run(&f, &query)?;
    let body: Output = serde_json::from_str(&text)?;
    assert_eq!(body.find.coverage.state, "complete");
    assert_eq!(body.find.coverage.total_files, 6);
    assert_eq!(body.find.hits, 7);
    assert!(body.rows.iter().all(|r| r.path.starts_with("src/")));
    Ok(())
}

#[test]
fn structured_check_returns_multiple_spanned_errors_at_exit_one() -> Result<()> {
    let f = fixture()?;
    let file = f.root.join("broken.tuft");
    fs::write(
        &file,
        "@h := find text ;\n@v := project @h [oops];\nemit @v;\n",
    )?;
    let result = outcome(&[
        "check".into(),
        file.to_string_lossy().into_owned(),
        "--json".into(),
    ]);
    assert_eq!(result.code, 1);
    assert!(result.stderr.is_empty());
    let body: Check = serde_json::from_str(&result.stdout)?;
    assert!(!body.ok);
    assert!(body.diagnostics.len() >= 2, "{}", result.stdout);
    assert_eq!(body.diagnostics[0].line, 1);
    assert_eq!(body.diagnostics[1].line, 2);
    for d in body.diagnostics {
        assert!(d.span.lo <= d.span.hi);
    }
    Ok(())
}
