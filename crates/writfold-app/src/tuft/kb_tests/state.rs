//! `tuft run` with no `--state`: inside a KB (or with `--kb`) Tuft keeps its
//! own state in the KB's Git directory, made once with only `evidence/`
//! and no trial key; with no KB and no state the refusal names both ways.
use super::super::cli::{Context, run_command};
use super::super::state::{Chosen, DIR, choose_from};
use super::receipts::start_run;
use super::{NOW, fixture, kb, one};
use crate::trial::Result;
use crate::tuft::tests::Fixture;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The one-hit program of `one`, run with no `--state`.
fn bare(f: &Fixture, extra: &[&str], cwd: &Path) -> Result<(String, Vec<String>)> {
    let file = f.root.join("bare.tuft");
    let program = format!(
        "@h := find text \"run(1)\" in \"src/\" at \"{}\";\n@e := retain @h around lines 0;\n\
@v := project @e [path, line, excerpt(max_chars: 40)];\nemit @v;\n",
        f.commit
    );
    fs::write(&file, program)?;
    let mut args: Vec<String> = ["run", &file.to_string_lossy(), "--repo"]
        .map(str::to_owned)
        .to_vec();
    args.push(f.repo.to_string_lossy().into_owned());
    args.extend(extra.iter().map(|s| s.to_string()));
    let context = Context {
        cwd: cwd.to_path_buf(),
        discover: cwd.join("inventory/assets").is_dir(),
        now: Some(NOW),
    };
    run_command(&args, context)
}

/// Every path under `dir`, relative and sorted.
fn tree(dir: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(next) = todo.pop() {
        for entry in fs::read_dir(&next)? {
            let path = entry?.path();
            out.push(path.strip_prefix(dir)?.to_string_lossy().into_owned());
            if path.is_dir() {
                todo.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// An operator's `WRITFOLD_STATE` or `WRITFOLD_KB` would choose for these
/// runs, so they are checked only without them.
fn environment_is_clear() -> bool {
    ["WRITFOLD_STATE", "WRITFOLD_KB"]
        .iter()
        .all(|v| std::env::var_os(v).is_none())
}

#[test]
fn a_fresh_kb_needs_no_setup_and_keeps_one_state() -> Result<()> {
    if !environment_is_clear() {
        return Ok(());
    }
    let f = fixture()?;
    let kb = kb(&f)?;
    let run = start_run(&kb)?;
    let state = kb.join(".git/writfold").join(DIR);
    assert!(!state.exists());
    let (shown, notes) = bare(&f, &["--cite"], &kb)?;
    assert!(
        shown.starts_with("src/lib.rs:3: run(1);\n  cite: "),
        "{shown}"
    );
    let recorded = format!("recorded runs/{run}/retrievals/");
    assert!(notes.iter().any(|n| n.starts_with(&recorded)), "{notes:?}");
    let receipts = kb.join(format!("runs/{run}/retrievals"));
    assert_eq!(fs::read_dir(&receipts)?.count(), 1);
    // Only `evidence/`, owner-only, holding the binding: no key or policy.
    let made = tree(&state)?;
    assert_eq!(made.first().map(String::as_str), Some("evidence"));
    assert!(made.iter().skip(1).all(|p| p.starts_with("evidence/")));
    assert_eq!(made.len(), 4, "{made:?}"); // .json, .bytes, .line
    assert_eq!(fs::metadata(&state)?.permissions().mode() & 0o777, 0o700);
    assert!(!tree(&kb)?.iter().any(|p| p.ends_with("operator.pem")));
    // The trial state binds the same bytes to the same digest.
    let (_, _) = one(&f, &[], &f.root)?;
    let evidence = tree(&state.join("evidence"))?;
    assert_eq!(evidence, tree(&f.state.join("evidence"))?);
    // A second run reuses the state: same directory, same bindings.
    let inode = fs::metadata(&state)?.ino();
    let (again, _) = bare(&f, &["--cite"], &kb)?;
    assert_eq!(again, shown);
    assert_eq!((fs::metadata(&state)?.ino(), tree(&state)?), (inode, made));
    assert_eq!(fs::read_dir(&receipts)?.count(), 2);
    Ok(())
}

#[test]
fn an_explicit_kb_chooses_its_state_from_outside_it() -> Result<()> {
    if !environment_is_clear() {
        return Ok(());
    }
    let f = fixture()?;
    let kb = kb(&f)?;
    start_run(&kb)?;
    let (shown, _) = bare(&f, &["--kb", &kb.to_string_lossy()], &f.root)?;
    assert_eq!(shown.lines().next(), Some("src/lib.rs:3: run(1);"));
    assert!(kb.join(".git/writfold").join(DIR).join("evidence").is_dir());
    Ok(())
}

#[test]
fn no_kb_and_no_state_names_both_ways_forward() -> Result<()> {
    let home = PathBuf::from("/nonexistent/home");
    let none = || Ok(None);
    let Err(error) = choose_from(None, None, Some(&home), &none) else {
        return Err("chose a state with no KB and no state".into());
    };
    let message = error.to_string();
    for part in [
        "--kb DIR",
        "--state DIR",
        "WRITFOLD_STATE",
        "writfold trial init",
    ] {
        assert!(message.contains(part), "{message}");
    }
    assert!(message.contains("/nonexistent/home/.local/share/writfold/trial"));
    // A given state is the trial state, as before, and never looks for a KB.
    let f = fixture()?;
    let unused = || Err("looked for a KB".into());
    let given = f.state.to_string_lossy().into_owned();
    let chosen = choose_from(Some(&given), None, Some(&home), &unused)?;
    assert_eq!(chosen, Chosen::Trial(f.state.clone()));
    let chosen = choose_from(None, Some(&f.state), Some(&home), &unused)?;
    assert_eq!(chosen, Chosen::Trial(f.state.clone()));
    // Through the command, outside any KB.
    let default = std::env::var_os("HOME")
        .map(|h| Path::new(&h).join(".local/share/writfold/trial/policy.json"));
    if environment_is_clear() && !default.is_some_and(|p| p.is_file()) {
        let refused = bare(&f, &[], &f.root);
        assert!(
            refused.is_err_and(|e| e.to_string().starts_with("no state for Tuft:")),
            "outside a KB with no state"
        );
    }
    Ok(())
}
