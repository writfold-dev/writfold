//! Two clones of one repository on one machine: the second clone's gates,
//! guards, plans and watch judge the second clone, never the first one the
//! run was started in (its recorded worktree). The recorded worktree is
//! still the subject when the KB is kept in its own repository.

mod second_clone;

use second_clone::{Base, Result, WRITFOLD, put, two_clones};

/// The row of `gate status` (or a `watch` pane) for gate `unit`.
fn unit_row(text: &str) -> Result<String> {
    Ok(text
        .lines()
        .find(|l| l.starts_with("gate unit ["))
        .ok_or_else(|| format!("no unit row:\n{text}"))?
        .to_owned())
}

#[test]
fn a_second_clone_reads_one_repository_and_one_name() -> Result {
    let base = Base::new("second-clone-resume")?;
    let (_, b, run) = two_clones(&base, "", true)?;
    for args in [&["resume"][..], &["gate", "status", "--run", &run]] {
        let (text, _) = base.kb(&b, args)?;
        assert!(unit_row(&text)?.contains("[satisfied]"), "{text}");
        // One repository, one name: no definition read from clone A, so no
        // "defined in both" row naming it `demo:` beside `o@…`, and no
        // `next:` pointing at a file in the other clone.
        assert!(!text.contains("defined in both"), "{text}");
        assert!(!text.contains("demo:"), "{text}");
        assert!(!text.contains("kb lint"), "{text}");
    }
    Ok(())
}

#[test]
fn an_edit_in_the_second_clone_is_judged_there_by_every_reader() -> Result {
    let base = Base::new("second-clone-edit")?;
    let (_, b, run) = two_clones(&base, "", true)?;
    put(&b.join("src/l.rs"), "250\n")?;
    let (text, _) = base.kb(&b, &["gate", "status", "--run", &run])?;
    assert!(unit_row(&text)?.contains("[stale]"), "{text}");
    let (text, code) = base.kb(&b, &["gate", "lines", "--run", &run])?;
    assert!(text.contains("0 of 1 satisfied"), "{text}");
    assert_eq!(code, 2, "{text}");
    let (text, _) = base.kb(&b, &["resume"])?;
    assert!(unit_row(&text)?.contains("[stale]"), "{text}");
    let (text, _) = base.kb(&b, &["plan", "status", "--run", &run])?;
    assert!(text.contains("unit [stale]"), "{text}");
    let (text, code) = base.kb(&b, &["plan", "advance", "--run", &run])?;
    assert_eq!(code, 3, "{text}");
    let (text, _) = base.kb(&b, &["guard", "status"])?;
    assert!(text.contains("owes:") && text.contains("unit"), "{text}");
    let (text, code) = base.run(&b, &["watch", "--run", &run, "--view", "gates", "--once"])?;
    assert_eq!(code, 0, "{text}");
    assert!(unit_row(&text)?.contains("[stale]"), "{text}");
    assert!(text.contains("+dirty:1"), "{text}");
    Ok(())
}

#[test]
fn a_linked_worktree_of_the_second_clone_is_judged_as_itself() -> Result {
    let base = Base::new("second-clone-worktree")?;
    let (_, b, run) = two_clones(&base, "", true)?;
    let wt = base.0.join("wt");
    base.git(
        &b,
        &["worktree", "add", "-q", &wt.to_string_lossy(), "-b", "side"],
    )?;
    put(&wt.join("src/l.rs"), "250\n")?;
    let (text, _) = base.kb(&wt, &["gate", "status", "--run", &run])?;
    assert!(unit_row(&text)?.contains("[stale]"), "{text}");
    assert!(!text.contains("defined in both"), "{text}");
    Ok(())
}

#[test]
fn a_gate_run_from_the_second_clone_runs_there() -> Result {
    let base = Base::new("second-clone-run")?;
    let (a, b, run) = two_clones(&base, "kb", false)?;
    put(&b.join("src/l.rs"), "250\n")?;
    base.git(&b, &["commit", "-q", "-am", "B breaks it"])?;
    let kb = b.join("kb");
    let (text, code) = base.kb(&kb, &["gate", "run", "unit", "--run", &run])?;
    assert_eq!(base.ran_in()?, b, "{text}");
    assert_ne!(base.ran_in()?, a, "{text}");
    assert!(unit_row(&text)?.contains("[failed]"), "{text}");
    assert_ne!(code, 0, "{text}");
    let (text, code) = base.kb(&kb, &["gate", "lines", "--run", &run])?;
    assert!(text.contains("0 of 1 satisfied"), "{text}");
    assert_eq!(code, 2, "{text}");
    Ok(())
}

#[test]
fn a_kb_in_its_own_repository_still_judges_the_recorded_worktree() -> Result {
    let base = Base::new("second-clone-own-kb")?;
    let kb = base.0.join("kb");
    let code = base.0.join("code");
    for dir in [&kb, &code] {
        std::fs::create_dir_all(dir)?;
        base.git(dir, &["init", "-q"])?;
    }
    let (text, exit) = base.kb(&kb, &["init"])?;
    assert_eq!(exit, 0, "{text}");
    put(&code.join("src/l.rs"), "500\n")?;
    put(
        &kb.join(".writfold/gates/unit.toml"),
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 60\nscope = [\"src/**\"]\n",
    )?;
    for dir in [&kb, &code] {
        base.git(dir, &["add", "-A"])?;
        base.git(dir, &["commit", "-q", "-m", "start"])?;
    }
    let in_code = |args: &[&str]| -> Result<(String, i32)> {
        let out = base
            .command(WRITFOLD, &code)
            .env("WRITFOLD_KB", &kb)
            .arg("kb")
            .args(args)
            .output()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        Ok((text, out.status.code().unwrap_or(-1)))
    };
    let args = [
        "checkpoint",
        "--new-run",
        "--project",
        "demo",
        "--task",
        "t",
    ];
    let (text, exit) = in_code(&[&args[..], &["--note", "n"]].concat())?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = in_code(&["gate", "run", "unit"])?;
    assert_eq!(exit, 0, "{text}");
    put(&code.join("src/l.rs"), "250\n")?;
    // From the KB clone, the subject is the recorded code worktree.
    let (text, _) = base.kb(&kb, &["gate", "status"])?;
    assert!(unit_row(&text)?.contains("[stale]"), "{text}");
    assert!(unit_row(&text)?.contains("at code@"), "{text}");
    Ok(())
}
