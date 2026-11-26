//! The receiver's pre-receive hook (#64,
//! `docs/guide/recipes/receiver/pre-receive`), on a bare repository
//! (`docs/guide/recipes/receiver-rehearsal.sh`): a candidate that passes the
//! required gate lands; a candidate that fails it is refused whether it
//! weakens the gate's definition, drops it from the required set, or
//! carries a forged receipt and a plan that skips it, because the receiver
//! runs the policy revision's gates itself into its own KB; a push to
//! another branch is not judged.
use std::path::Path;
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Run the rehearsal; its whole output.
fn rehearse() -> Result<String> {
    let dir = std::env::temp_dir().join(format!("writfold-receiver-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("sh")
        .arg(root.join("docs/guide/recipes/receiver-rehearsal.sh"))
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

#[test]
fn the_hook_lands_a_passing_candidate_and_refuses_every_way_around_the_gate() -> Result {
    let text = rehearse()?;
    let section = |title: &str| -> String {
        (text.split("\n## ").find(|s| s.starts_with(title)))
            .unwrap_or_default()
            .to_owned()
    };
    let landed = |s: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix("landed="))
            .map(str::to_owned)
    };
    let first = landed(&section("a candidate that passes"));
    assert!(
        section("a candidate that passes").contains("handoff=accepted"),
        "{text}"
    );
    for title in [
        "a candidate that fails and weakens the gate",
        "a candidate that drops the gate from the required set",
        "a candidate that carries a forged receipt",
    ] {
        let s = section(title);
        assert!(
            s.contains("handoff test [refused] not-passed"),
            "{title}: {s}"
        );
        assert!(s.contains("pre-receive: refused:"), "{title}: {s}");
        assert_eq!(landed(&s), first, "{title}: main moved\n{s}");
    }
    let wip = section("a push to another branch");
    assert!(!wip.contains("pre-receive"), "{wip}");
    assert!(!wip.contains("wip= "), "{wip}");
    Ok(())
}
