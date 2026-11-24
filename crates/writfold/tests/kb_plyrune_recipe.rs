//! The optional Plyrune gate of `docs/guide/recipes/` (#106) runs as
//! published, against the stand-in checker in `fixtures/plyrune/` (not
//! Plyrune, not its agreed profile). No case but a kept report with exit 0
//! satisfies the gate; a changed requirement or a new in-scope file makes
//! it stale; a fresh clone reads it from the committed evidence, and
//! without that evidence never reads it as met.
use std::path::Path;
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn only_a_kept_report_with_a_clean_exit_satisfies_the_gate() -> Result {
    let dir = std::env::temp_dir().join(format!("writfold-plyrune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("sh")
        .arg(root.join("docs/guide/recipes/plyrune-rehearsal.sh"))
        .arg(&dir)
        .arg(root.join("fixtures/plyrune/bin"))
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
        .env_remove("PLYRUNE")
        .env_remove("PLYRUNE_PIN")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&bin);
    assert!(out.status.success(), "{text}");
    let section = |title: &str| -> String {
        text.split("\n## ")
            .find(|s| s.starts_with(title))
            .unwrap_or_default()
            .to_owned()
    };
    let cases = [
        ("no checker", "[failed] mechanical fail exit=127"),
        (
            "checker at another version",
            "[failed] mechanical fail exit=3",
        ),
        ("checker says no", "[failed] mechanical fail exit=1"),
        (
            "checker exits 0 but writes no report",
            "[unknown] mechanical unknown exit=0",
        ),
        ("checker crashes", "[failed] mechanical fail exit=137"),
        ("checker satisfied", "[satisfied] mechanical pass exit=0"),
        (
            "requirement edited",
            "why: changed in scope: requirements/r1.md",
        ),
        ("new in-scope file", "why: changed in scope: src/extra.rs"),
        ("fresh clone with the evidence", "gate plyrune [satisfied]"),
        (
            "fresh clone without the evidence",
            "retained evidence report.json is not here",
        ),
    ];
    for (title, want) in cases {
        let s = section(title);
        assert!(s.contains(want), "{title}: want {want:?}\n{s}");
    }
    assert!(section("no checker").contains("report.json was not kept"));
    assert!(!section("fresh clone without the evidence").contains("[satisfied]"));
    Ok(())
}
