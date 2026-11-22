//! The clone coverage recipe of `docs/guide/recipes/` (#89) runs as
//! published on a disposable KB citing three repositories: `alpha` is
//! approved and provisioned (shallow, then measured against a full mirror),
//! `beta` is named by a gate citation and so heads the shortlist, but is
//! deliberately left missing, and `gamma` is a partial clone, which
//! `kb repos --missing` does not list. Nothing before `provision` touches
//! the network or writes to Git.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Demo(PathBuf);

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sh(dir: &Path, demo: &Demo, script: &str) -> Result<(String, i32)> {
    let path = format!(
        "{}:{}",
        demo.0.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("sh")
        .args(["-c", script])
        .current_dir(dir)
        .env("PATH", path)
        .env("DEMO", &demo.0)
        .env("RECIPE", recipe())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_KB_REPOS")
        .env_remove("WRITFOLD_SESSION")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

fn recipe() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/guide/recipes/clone-coverage.py")
        .display()
        .to_string()
}

/// Three source repositories with ten commits of history each, and a KB
/// citing them by unreachable `url=`s: alpha four times, beta three times
/// (one a gate citation), gamma twice. gamma is a partial clone, mapped.
const SETUP: &str = r#"set -e
for r in alpha beta gamma; do
  mkdir -p "$DEMO/src/$r" && cd "$DEMO/src/$r" && git init -q -b main
  git config uploadpack.allowFilter true
  for i in 1 2 3 4; do printf 'fn %s_%s() {}\n' $r $i > f$i.rs; done
  for n in 1 2 3 4 5 6 7 8 9 10; do
    head -c 60000 /dev/urandom | od -An -x > history.txt
    git add -A && git commit -qm "c$n"
  done
done
mkdir -p "$DEMO/kb/lantern" && cd "$DEMO/kb" && git init -q -b main && writfold kb init >/dev/null
cite() { printf '⟦cite kind=code repo=%s url=https://example.invalid/%s.git rev=%s path=%s L=1-1 blob=%s⟧\n' \
  "$1" "$1" "$(git -C "$DEMO/src/$1" rev-parse HEAD)" "$2" "$(git -C "$DEMO/src/$1" rev-parse "HEAD:$2")"; }
{ echo '# Coverage'; echo
  for f in f1.rs f2.rs f3.rs f4.rs; do cite alpha $f; done
  for f in f1.rs f2.rs; do cite beta $f; done
  printf '⟦cite kind=gate rev=beta@%s⟧\n' "$(git -C "$DEMO/src/beta" rev-parse HEAD)"
  for f in f1.rs f2.rs; do cite gamma $f; done
} > lantern/coverage.md
git add -A && git commit -qm kb
git clone -q --filter=blob:none "file://$DEMO/src/gamma" "$DEMO/partial/gamma"
mkdir -p .git/writfold && printf '# local repository map\ngamma %s\n' "$DEMO/partial/gamma/.git" > .git/writfold/repos
"#;

fn demo() -> Result<Demo> {
    let dir = std::env::temp_dir().join(format!("writfold-coverage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let d = Demo(dir.clone());
    std::fs::create_dir_all(dir.join("bin"))?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), dir.join("bin/writfold"))?;
    let (text, exit) = sh(&dir, &d, SETUP)?;
    assert_eq!(exit, 0, "{text}");
    Ok(d)
}

fn lines_with<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    text.lines().filter(|l| l.contains(needle)).collect()
}

#[test]
fn coverage_is_planned_reviewed_then_provisioned() -> Result {
    let d = demo()?;
    let kb = d.0.join("kb");
    let before = sh(&kb, &d, "git rev-parse HEAD; git status --porcelain")?.0;

    let (inventory, exit) = sh(&kb, &d, "python3 \"$RECIPE\" inventory")?;
    assert_eq!(exit, 0, "{inventory}");
    assert!(
        inventory.contains("# repos=3 missing=2 partial=1"),
        "{inventory}"
    );
    assert!(
        inventory.contains("--missing` does not list it"),
        "{inventory}"
    );
    let (missing, _) = sh(&kb, &d, "writfold kb repos --missing")?;
    assert!(!missing.contains("repo gamma"), "{missing}");

    let (shortlist, exit) = sh(&kb, &d, "python3 \"$RECIPE\" shortlist")?;
    assert_eq!(exit, 0, "{shortlist}");
    let order = lines_with(&shortlist, "  ");
    assert!(
        order[1].contains("beta") && order[1].contains("named by a gate citation"),
        "{shortlist}"
    );
    assert!(
        order[2].contains("alpha") && order[2].contains("token count"),
        "{shortlist}"
    );
    assert!(
        order[3].contains("gamma") && order[3].contains("partial"),
        "{shortlist}"
    );
    let (verdicts, _) = sh(
        &kb,
        &d,
        "python3 \"$RECIPE\" verdicts --save ../before.json",
    )?;
    assert!(
        verdicts.contains("unverifiable repo-not-local"),
        "{verdicts}"
    );
    assert!(
        verdicts.contains("unverifiable partial-clone"),
        "{verdicts}"
    );
    // Nothing so far wrote to Git or cloned anything.
    assert_eq!(
        sh(&kb, &d, "git rev-parse HEAD; git status --porcelain")?.0,
        before
    );
    assert!(!d.0.join("clones").exists());

    // The operator approves alpha only; beta stays missing on purpose.
    std::fs::write(
        d.0.join("approved.txt"),
        format!("alpha file://{}/src/alpha\n", d.0.display()),
    )?;
    let (provisioned, exit) = sh(
        &kb,
        &d,
        "python3 \"$RECIPE\" provision ../approved.txt --into ../clones --depth 1",
    )?;
    assert_eq!(exit, 0, "{provisioned}");
    assert!(
        provisioned.contains("alpha: ") && provisioned.contains("(depth 1)"),
        "{provisioned}"
    );
    assert!(!d.0.join("clones/beta.git").exists());
    let (sizes, _) = sh(
        &kb,
        &d,
        "git clone -q --mirror \"file://$DEMO/src/alpha\" ../full/alpha.git && du -sk ../clones/alpha.git ../full/alpha.git",
    )?;
    let kib: Vec<u64> = sizes
        .lines()
        .filter_map(|l| l.split_whitespace().next()?.parse().ok())
        .collect();
    assert!(
        kib.len() == 2 && kib[0] < kib[1],
        "shallow is smaller: {sizes}"
    );

    let _ = sh(&kb, &d, "python3 \"$RECIPE\" verdicts --save ../after.json")?;
    let (compare, _) = sh(
        &kb,
        &d,
        "python3 \"$RECIPE\" compare ../before.json ../after.json",
    )?;
    // alpha's four citations carry blob= at unchanged paths: fresh even shallow.
    assert!(
        compare
            .lines()
            .any(|l| l.ends_with("  fresh") && l.contains("     4")),
        "{compare}"
    );
    let local = lines_with(&compare, "repo-not-local");
    assert!(
        local
            .iter()
            .any(|l| l.split_whitespace().nth(1) == Some("3")),
        "{compare}"
    );
    assert!(compare.contains("unverifiable partial-clone"), "{compare}");
    Ok(())
}
