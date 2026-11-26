//! The Claude Code adapter (#150, `plugins/writfold/adapter/claude_code.py`)
//! drives the research contract end to end with a fake harness
//! (`docs/guide/recipes/claude-code-rehearsal.sh`): prepare, a deterministic
//! export, run, deliver bound to the invocation, check, cover. What the
//! adapter observed is a statement (`context.submitted`, attested by
//! `adapter:claude-code@…`), never a check. Export refuses, rather than
//! drops: an unknown key of the record, a tool with no mapping, a model
//! hint Claude Code does not take; `--model` states a substitution.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Run the rehearsal into `dir`; its whole output.
fn rehearse(dir: &Path, bin: &Path) -> Result<String> {
    let out = Command::new("sh")
        .arg(root().join("docs/guide/recipes/claude-code-rehearsal.sh"))
        .arg(dir)
        .env("PATH", path(bin))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_SESSION")
        .env_remove("ADAPTER_MODEL")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{text}");
    Ok(text)
}

fn path(bin: &Path) -> String {
    format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

/// `claude_code.py export PKG [ARGS…]`: exit code and output.
fn export(pkg: &Path, args: &[&str]) -> Result<(i32, String)> {
    let out = Command::new("python3")
        .arg(root().join("plugins/writfold/adapter/claude_code.py"))
        .arg("export")
        .arg(pkg)
        .args(args)
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((out.status.code().unwrap_or(-1), text))
}

/// A copy of `pkg` whose record is changed by `edit`.
fn variant(pkg: &Path, to: &Path, edit: impl Fn(&mut serde_json::Value)) -> Result<PathBuf> {
    std::fs::create_dir_all(to)?;
    for name in ["instructions.md", "input"] {
        std::fs::copy(pkg.join(name), to.join(name))?;
    }
    let mut record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(pkg.join("invocation.json"))?)?;
    edit(&mut record);
    std::fs::write(to.join("invocation.json"), record.to_string())?;
    Ok(to.to_path_buf())
}

fn refusals(dir: &Path) -> Result {
    let pkg = dir.join("pkg");
    let extra = variant(&pkg, &dir.join("extra"), |r| {
        r["harness_hint"] = "x".into();
    })?;
    let (code, text) = export(&extra, &[])?;
    assert_eq!(code, 3, "{text}");
    assert!(text.contains("does not know: harness_hint"), "{text}");
    let tool = variant(&pkg, &dir.join("tool"), |r| {
        r["definitions"][2]["id"] = "web.fetch".into();
    })?;
    let (code, text) = export(&tool, &[])?;
    assert_eq!(code, 3, "{text}");
    assert!(
        text.contains("tool web.fetch has no Claude Code mapping"),
        "{text}"
    );
    let model = variant(&pkg, &dir.join("model"), |r| {
        r["model"] = "reviewer".into();
    })?;
    let (code, text) = export(&model, &[])?;
    assert_eq!(code, 3, "{text}");
    assert!(
        text.contains("model hint 'reviewer' is not a Claude Code model"),
        "{text}"
    );
    let (code, text) = export(&model, &["--model", "haiku"])?;
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("Stated: model hint 'reviewer' substituted with 'haiku'"),
        "{text}"
    );
    Ok(())
}

#[test]
fn the_adapter_drives_a_unit_end_to_end_and_refuses_what_it_would_drop() -> Result {
    let dir = std::env::temp_dir().join(format!("writfold-adapter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = dir.with_extension("bin");
    let _ = std::fs::remove_dir_all(&bin);
    std::fs::create_dir_all(&bin)?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), bin.join("writfold"))?;
    let text = rehearse(&dir, &bin)?;
    for want in [
        "export=deterministic",
        "\"Grep\"",
        "check: invocation.bound [satisfied]",
        "checklist 1/1 units covered",
        "check: context.submitted [attested] attested by adapter:claude-code@",
    ] {
        assert!(text.contains(want), "{want:?} not in\n{text}");
    }
    refusals(&dir)?;
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&bin);
    Ok(())
}
