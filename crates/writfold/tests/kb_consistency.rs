//! The KB-consistency research contract (#157) runs end to end with two
//! fake harnesses of different shapes and no model
//! (`docs/guide/recipes/kb-consistency-rehearsal.sh` over
//! `fixtures/kb/kb-consistency/`). Each attempt to skip required work is
//! refused by the gate, check or row that should refuse it; the whole flow
//! completes the plan; a wrong, well-formed report covers its unit
//! (coverage is not correctness); a report checked again after its step is
//! done still passes, and a withdrawn one grounds nothing (#164); `--out`
//! inside the KB and `--for` an unknown unit are refused (#167); a record
//! added makes the scope stale, and once it runs again only the new units
//! are owed; an edited member stales only its units, the edited seed all
//! of them (#166).
use std::path::Path;
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Run the rehearsal; its whole output.
fn rehearse() -> Result<String> {
    let dir = std::env::temp_dir().join(format!("writfold-consistency-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("sh")
        .arg(root.join("docs/guide/recipes/kb-consistency-rehearsal.sh"))
        .arg(&dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_SESSION")
        .env_remove("WRITFOLD_KB_WORK")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&bin);
    assert!(out.status.success(), "{text}");
    Ok(text)
}

/// Each section of the rehearsal, and what it must show.
const CASES: [(&str, &str); 34] = [
    (
        "prepare before the scope",
        "needs gate related-scope, which is not met: gate related-scope [open]",
    ),
    (
        "scope walk stopped",
        "gate related-scope [failed] mechanical fail exit=2",
    ),
    (
        "scope walk stopped",
        "needs gate related-scope, which is not met: gate related-scope [failed]",
    ),
    ("enumerate", "units=10"),
    // Both harness shapes give the same standings.
    ("ten prepares", "candidates=10 ready=10 satisfied=10"),
    ("eight of ten covered", "checklist 8/10 units covered"),
    ("eight of ten covered", "not advanced"),
    ("a report that names no invocation", "line 1 no-invocation"),
    (
        "a report that names no invocation",
        "is not grounded (unbound)",
    ),
    ("a report prepared at another step", "line 1 another-step"),
    (
        "a report prepared at another step",
        "is not grounded (unbound)",
    ),
    (
        "a report prepared for another unit",
        "takes its own evidence",
    ),
    ("the skill edited after prepare", "line 1 closure-moved"),
    (
        "a statement the harness cannot make",
        "check: context.submitted [open] attested",
    ),
    ("a statement the harness cannot make", "[blocked]"),
    ("a wrong report", "invocation.bound [satisfied]"),
    ("a wrong report", "checklist 9/10 units covered"),
    ("all covered", "plan complete-with-exceptions"),
    // #168: the disposition is still owed after every unit is covered.
    ("all covered", "gate findings-disposition [open]"),
    ("all covered", "checklist 10/10 units covered"),
    // #164: the binding is judged at the step the report recorded.
    (
        "a report checked again",
        "check: invocation.bound [satisfied]",
    ),
    ("a report checked again", "checklist 10/10 units covered"),
    // #164: a withdrawn report grounds nothing.
    ("a report withdrawn", "cited evidence moved: delivery:"),
    ("a report withdrawn", "was ready, now withdrawn"),
    ("a report withdrawn", "checklist 9/10 units covered"),
    ("a report withdrawn", "is not grounded (withdrawn)"),
    (
        "a record added",
        "why: the scope gate related-scope is stale",
    ),
    // #166: a member added leaves earlier units covered.
    ("a record added", "checklist 9/15 units covered"),
    ("one member edited", "checklist 6/15 units covered"),
    ("one member edited", "decision-a changed since this row"),
    ("the seed edited", "checklist 0/15 units covered"),
    ("the seed edited", "the seed changed since this row"),
    // #167: two refusals at prepare.
    ("--out inside the KB", "is inside the KB; next:"),
    ("--for an unknown unit", "pair-review has no such item now"),
];

#[test]
fn every_skipped_step_is_refused_and_the_whole_flow_completes() -> Result {
    let text = rehearse()?;
    let section = |title: &str| -> String {
        (text.split("\n## ").find(|s| s.starts_with(title)))
            .unwrap_or_default()
            .to_owned()
    };
    for (title, want) in CASES {
        let s = section(title);
        assert!(s.contains(want), "{title}: want {want:?}\n{s}");
    }
    assert!(!section("a statement the harness cannot make").contains("[ready]"));
    // #168: both owed units are named, not only the next one.
    let owed = section("eight of ten covered");
    assert_eq!(
        owed.matches("[open] proposal/decision/").count(),
        2,
        "{owed}"
    );
    // #168: one unit through both harness shapes reads the same.
    let both = section("one unit, both harnesses");
    let lines: Vec<&str> = both.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 4, "{both}");
    assert_eq!(lines[..2], lines[2..], "{both}");
    assert!(both.contains("invocation.bound [satisfied]"), "{both}");
    Ok(())
}
