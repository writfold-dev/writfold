//! `kb init`: create-new only. A path that exists is kept and listed;
//! `--dry-run` writes nothing; a directory inside another KB is refused;
//! `--example research-policy` writes the bytes of the fixture. With no
//! policy configured, `resume` says so in one line.
mod support;

use std::path::{Path, PathBuf};
use support::{NOW, TestResult};
use writfold_kb::init::Init;
use writfold_kb::{Request, Verb};

fn scratch(name: &str) -> TestResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!("writfold-init-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    Ok(std::fs::canonicalize(dir)?)
}

fn request(cwd: &Path, verb: Verb) -> Request {
    Request {
        verb,
        kb: None,
        cwd: cwd.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

fn init(cwd: &Path, example: Option<&str>, dry_run: bool) -> TestResult<(String, u8)> {
    let verb = Verb::Init(Init {
        dir: None,
        example: example.map(str::to_owned),
        dry_run,
    });
    let out = writfold_kb::run(&request(cwd, verb))?;
    Ok((out.text, out.exit))
}

#[test]
fn init_creates_the_marker_and_never_overwrites() -> TestResult {
    let dir = scratch("plain")?;
    let refused = writfold_kb::run(&request(&dir, Verb::IndexStatus));
    assert!(refused.is_err_and(|e| e.to_string().contains("create one here: writfold kb init")));
    let (text, exit) = init(&dir, None, true)?;
    assert!(
        exit == 0 && text.contains("wrote inventory/assets/.gitkeep"),
        "{text}"
    );
    assert!(!dir.join("inventory").exists(), "a dry run writes nothing");
    let (text, exit) = init(&dir, None, false)?;
    assert!(exit == 0 && text.contains("wrote=1 kept=0"), "{text}");
    assert!(dir.join("inventory/assets/.gitkeep").is_file());
    std::fs::create_dir_all(dir.join(".writfold/plans"))?;
    let mine = "schema = \"writfold-plan-required/1\"\nstart = []\n";
    std::fs::write(dir.join(".writfold/plans/required.toml"), mine)?;
    let (text, exit) = init(&dir, Some("research-policy"), false)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("kept  inventory/assets/.gitkeep (exists; not changed)"),
        "{text}"
    );
    assert!(
        text.contains("kept  .writfold/plans/required.toml"),
        "{text}"
    );
    assert!(
        text.contains("wrote .writfold/gates/research.toml"),
        "{text}"
    );
    assert!(
        text.contains("these files are yours to edit or delete"),
        "{text}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".writfold/plans/required.toml"))?,
        mine
    );
    let fixture = support::fixture("research-policy").join(".writfold/gates/research.md");
    assert_eq!(
        std::fs::read(dir.join(".writfold/gates/research.md"))?,
        std::fs::read(fixture)?
    );
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn init_inside_another_kb_is_refused() -> TestResult {
    let dir = scratch("nested")?;
    init(&dir, None, false)?;
    let inner = dir.join("sub");
    std::fs::create_dir_all(&inner)?;
    let (text, exit) = init(&inner, None, false)?;
    assert!(exit == 3 && text.contains("is inside the KB at"), "{text}");
    assert!(!inner.join("inventory").exists());
    let unknown = writfold_kb::run(&request(
        &dir,
        Verb::Init(Init {
            dir: None,
            example: Some("nothing".into()),
            dry_run: false,
        }),
    ));
    assert!(unknown.is_err_and(|e| e.to_string().contains("unknown example nothing")));
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// After `init` in a plain repository, a first checkpoint and `resume`
/// work, and `resume` says that no policy is configured.
#[test]
fn a_first_run_resumes_and_says_no_policy_is_configured() -> TestResult {
    let dir = scratch("first")?;
    support::git(&dir, &["init", "-q"])?;
    init(&dir, None, false)?;
    let checkpoint = Verb::Write(writfold_kb::write::Write::Checkpoint(
        writfold_kb::write::run::Checkpoint {
            artifacts: Vec::new(),
            run: None,
            new_run: Some(writfold_kb::write::run::NewRun {
                project: "demo".into(),
                task: "try it".into(),
                mode: None,
            }),
            note: "first note".into(),
        },
    ));
    let out = writfold_kb::run(&request(&dir, checkpoint))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    let out = writfold_kb::run(&request(&dir, Verb::Resume { run: None }))?;
    assert!(
        out.text
            .contains("# note: policy: none configured (continuity only"),
        "{}",
        out.text
    );
    std::fs::create_dir_all(dir.join(".writfold/gates"))?;
    std::fs::write(dir.join(".writfold/gates/x.toml"), "")?;
    let out = writfold_kb::run(&request(&dir, Verb::Resume { run: None }))?;
    assert!(
        !out.text.contains("policy: none configured"),
        "{}",
        out.text
    );
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn init_writes_the_delivery_example() -> TestResult {
    let dir = scratch("delivery")?;
    let (text, exit) = init(&dir, Some("delivery"), false)?;
    assert!(
        exit == 0 && text.contains("wrote .writfold/deliveries/guard.toml"),
        "{text}"
    );
    assert!(text.contains("the built-in modes are unchanged"), "{text}");
    let fixture = support::fixture("delivery-example");
    for file in [
        ".writfold/deliveries/deny-external.toml",
        ".writfold/deliveries/deny-internal.toml",
        ".writfold/deliveries/commit-message.toml",
        ".writfold/deliveries/release-note.toml",
        ".writfold/deliveries/guard.toml",
        ".writfold/gates/commit-format.toml",
        ".writfold/gates/commit_format.sh",
        ".writfold/plans/ship.toml",
        ".writfold/deliveries/design-doc.toml",
        ".writfold/deliveries/vale-house.toml",
        ".writfold/vale/house/.vale.ini",
        ".writfold/vale/house/styles/House/Placeholders.yml",
    ] {
        assert_eq!(
            std::fs::read(dir.join(file))?,
            std::fs::read(fixture.join(file))?,
            "{file}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
