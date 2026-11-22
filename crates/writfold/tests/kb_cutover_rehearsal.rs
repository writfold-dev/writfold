//! The cutover rehearsal of `docs/guide/recipes/` (#96) runs as published:
//! the author's own `gate lines` reads complete with exceptions, while the
//! receiver's `handoff check` against the author's run refuses a gate run
//! under a weakened definition and a gate the plan skipped; the receiver's
//! own run, with the policy revision's definitions, is accepted; the landed
//! decision reaches `applied` and `verified`.
use std::path::Path;
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn the_receiver_refuses_the_authors_evidence_then_accepts_its_own() -> Result {
    let dir = std::env::temp_dir().join(format!("writfold-cutover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let recipe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/recipes/cutover-rehearsal.sh");
    let out = Command::new("sh")
        .arg(&recipe)
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
    let author = section("author");
    assert!(author.contains("[accepted] Report status"), "{author}");
    assert!(
        author.contains("2 of 3 satisfied, 1 skipped; plan complete-with-exceptions"),
        "{author}"
    );
    let judged = section("receiver: judge");
    assert!(judged.contains("handoff build [accepted]"), "{judged}");
    assert!(
        judged.contains("handoff test [refused] definition-changed"),
        "{judged}"
    );
    assert!(
        judged.contains("handoff lint [refused] no-evidence"),
        "{judged}"
    );
    assert!(judged.contains("handoff=refused"), "{judged}");
    let own = section("receiver: run");
    for gate in ["build", "test", "lint"] {
        assert!(own.contains(&format!("handoff {gate} [accepted]")), "{own}");
    }
    assert!(own.contains("handoff=accepted"), "{own}");
    let landing = section("landing");
    assert!(landing.contains("[applied] Report status"), "{landing}");
    assert!(landing.contains("[verified] Report status"), "{landing}");
    Ok(())
}
