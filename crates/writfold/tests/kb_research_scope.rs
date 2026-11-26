//! The research-scope gate of `docs/guide/recipes/` (#155) runs as
//! published: `kb impact --manifest` kept by a mechanical gate. A full read
//! is satisfied and lists the members; the walk stopped at its work limit
//! fails; a timeout is unknown; a record added that references the seed
//! makes the receipt stale and the next run lists it; a clone without the
//! kept manifest never reads it as met; a seed nothing references is an
//! empty, complete manifest.
use std::path::Path;
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn the_scope_gate_holds_only_a_complete_kept_walk() -> Result {
    let dir = std::env::temp_dir().join(format!("writfold-scope-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("sh")
        .arg(root.join("docs/guide/recipes/research-scope-rehearsal.sh"))
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
    let section = |title: &str| -> String {
        (text.split("\n## ").find(|s| s.starts_with(title)))
            .unwrap_or_default()
            .to_owned()
    };
    let cases = [
        ("scope read in full\n", "[satisfied] mechanical pass exit=0"),
        (
            "scope read in full\n",
            "members=5 \"coverage\":\"complete\"",
        ),
        (
            "walk stopped at its work limit",
            "[failed] mechanical fail exit=2",
        ),
        ("walk timed out", "[unknown] mechanical unknown timeout"),
        ("a record added", "[stale]"),
        (
            "a record added",
            "why: changed in scope: proposals/decision/",
        ),
        ("a record added", "members=6 \"coverage\":\"complete\""),
        (
            "fresh clone with the evidence",
            "gate related-scope [satisfied]",
        ),
        (
            "fresh clone without the evidence",
            "retained evidence manifest.json is not here",
        ),
        (
            "a seed nothing references",
            "members=0 \"coverage\":\"complete\"",
        ),
    ];
    for (title, want) in cases {
        let s = section(title);
        assert!(s.contains(want), "{title}: want {want:?}\n{s}");
    }
    assert!(!section("fresh clone without the evidence").contains("[satisfied]"));
    Ok(())
}
