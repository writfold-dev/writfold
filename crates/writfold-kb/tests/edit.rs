//! The Splicery adapter over the fixture world: recipes replayed against
//! the code repository (`demo-service`) recorded as the run's worktree,
//! previews, multi-file batches, refusals that write nothing, receipts in
//! `resume`, and G1 gates made stale by an edit.
mod support;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::edit::EditVerb;
use writfold_kb::error::KbError;
use writfold_kb::gate::GateVerb;
use writfold_kb::write::Step;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const RETRY: &str =
    "edit \"src/retry.rs\" {\n  replace one text \"attempts = 5\" with \"attempts = 7\"\n}\n";
const LEDGER: &str = "edit \"src/ledger.rs\" {\n  each @r in line_matches rx\"^(?<lead>[ ]+)append\\(row\\);$\" expect 1 {\n    replace @r with [keep(@r.lead), \"append(row)?;\"]\n  }\n  @doc := one line \"// ledger writes\"\n  after @doc insert \"\\n// every row is appended once\"\n}\n";

fn recipe(w: &World, name: &str, text: &str) -> TestResult<PathBuf> {
    let path = w.base.join("recipes").join(name);
    std::fs::create_dir_all(w.base.join("recipes"))?;
    std::fs::write(&path, text)?;
    Ok(path)
}

fn apply(w: &World, recipes: &[PathBuf], worktree: bool, dry_run: bool) -> EditVerb {
    EditVerb::Apply {
        recipes: recipes.to_vec(),
        run: Some(RUN.into()),
        worktree: worktree.then(|| w.code.clone()),
        dry_run,
    }
}

fn edit(w: &World, verb: EditVerb) -> writfold_kb::error::Result<(String, u8)> {
    let output = writfold_kb::run(&w.request(Verb::Edit(verb)))?;
    let text = output
        .text
        .replace(&w.base.display().to_string(), "<WORLD>");
    Ok((text, output.exit))
}

fn refused(result: writfold_kb::error::Result<(String, u8)>) -> TestResult<String> {
    match result {
        Err(KbError::Refused(why)) => Ok(why),
        other => Err(format!("expected a refusal, got {other:?}").into()),
    }
}

/// Receipt names end in six random hex digits.
fn stable(text: &str) -> String {
    let text = text.replace(RUN, "<RUN>");
    regex::Regex::new(r"(\d{8}T\d{6}Z)-[0-9a-f]{6}")
        .map(|re| re.replace_all(&text, "$1-XXXXXX").into_owned())
        .unwrap_or_default()
        .replace("<RUN>", RUN)
}

fn golden(name: &str, text: &str, exit: Option<u8>) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let mut actual = stable(text);
    if let Some(exit) = exit {
        actual.push_str(&format!("# exit={exit}\n"));
    }
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn snapshot(w: &World, paths: &[&str]) -> TestResult<Vec<Vec<u8>>> {
    Ok(paths
        .iter()
        .map(|p| std::fs::read(w.code.join(p)))
        .collect::<Result<_, _>>()?)
}

fn receipts(w: &World) -> Vec<PathBuf> {
    let dir = w.kb.join(format!("runs/{RUN}/edits"));
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// No dot-named temporary is left in the worktree's `src/`.
fn no_temporaries(w: &World) -> TestResult {
    for entry in std::fs::read_dir(w.code.join("src"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        assert!(!name.starts_with('.'), "leftover {name}");
    }
    Ok(())
}

#[test]
fn preview_then_multi_file_apply_and_resume() -> TestResult {
    let w = world(&[])?;
    let paths = ["src/retry.rs", "src/ledger.rs"];
    let original = snapshot(&w, &paths)?;
    let recipes = [
        recipe(&w, "retry.spl", RETRY)?,
        recipe(&w, "ledger.spl", LEDGER)?,
    ];
    let (text, exit) = edit(&w, apply(&w, &recipes, true, true))?;
    assert_eq!(exit, 0, "{text}");
    golden("edit-preview.txt", &text, Some(exit))?;
    assert_eq!(snapshot(&w, &paths)?, original, "a dry run wrote");
    assert!(receipts(&w).is_empty());
    assert!(!w.kb.join(format!("runs/{RUN}/writfold-run.json")).exists());

    let (text, exit) = edit(&w, apply(&w, &recipes, true, false))?;
    assert_eq!(exit, 0, "{text}");
    golden("edit-apply.txt", &text, Some(exit))?;
    let now = snapshot(&w, &paths)?;
    assert!(String::from_utf8(now[0].clone())?.contains("attempts = 7"));
    assert!(String::from_utf8(now[1].clone())?.contains("        append(row)?;\n"));
    no_temporaries(&w)?;
    let found = receipts(&w);
    assert_eq!(found.len(), 1);
    let receipt = std::fs::read_to_string(&found[0])?;
    golden(
        "edit-receipt.md",
        &receipt.replace(&w.base.display().to_string(), "<WORLD>"),
        None,
    )?;
    let sidecar = std::fs::read_to_string(w.kb.join(format!("runs/{RUN}/writfold-run.json")))?;
    assert!(sidecar.contains("\"worktree\""), "{sidecar}");

    let (text, exit) = w.run(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("edit ") && text.contains("splicery files=2 splices=3 at demo-service@"),
        "{text}"
    );
    assert!(text.contains("  file: src/retry.rs "), "{text}");
    let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");

    // The worktree is now recorded: a later apply needs no --worktree, and
    // another directory is refused.
    let again = [recipe(
        &w,
        "again.spl",
        &RETRY.replace("= 7", "= 9").replace("= 5", "= 7"),
    )?];
    let (_, exit) = edit(&w, apply(&w, &again, false, false))?;
    assert_eq!(exit, 0);
    let mut other = apply(&w, &again, false, false);
    if let EditVerb::Apply { worktree, .. } = &mut other {
        *worktree = Some(w.kb.clone());
    }
    assert!(refused(edit(&w, other))?.contains("records the worktree"));
    Ok(())
}

#[test]
fn replay_refusals_carry_match_counts_and_write_nothing() -> TestResult {
    let w = world(&[])?;
    let paths = ["src/retry.rs", "src/ledger.rs"];
    let original = snapshot(&w, &paths)?;
    let ambiguous = "edit \"src/ledger.rs\" {\n  replace one text \"row\" with \"item\"\n}\n";
    let counted = LEDGER.replace("expect 1", "expect 2");
    for (bad, wanted) in [
        (ambiguous, "matched 4 sites: line 2 bytes"),
        (
            counted.as_str(),
            "expected 2 matching lines, found 1: line 4 bytes",
        ),
    ] {
        let recipes = [recipe(&w, "retry.spl", RETRY)?, recipe(&w, "bad.spl", bad)?];
        let why = refused(edit(&w, apply(&w, &recipes, true, false)))?;
        assert!(
            why.contains("nothing was written") && why.contains(wanted),
            "{why}"
        );
        assert!(why.contains("recipe bad.spl"), "{why}");
        assert_eq!(snapshot(&w, &paths)?, original);
        assert!(receipts(&w).is_empty());
    }
    let why = refused(edit(
        &w,
        apply(&w, &[recipe(&w, "c.spl", &counted)?], true, false),
    ))?;
    assert!(why.contains("line 4: "), "{why}");
    let twice = [recipe(&w, "a.spl", RETRY)?, recipe(&w, "b.spl", RETRY)?];
    assert!(
        refused(edit(&w, apply(&w, &twice, true, false)))?
            .contains("two recipes edit src/retry.rs")
    );
    let outside = recipe(
        &w,
        "o.spl",
        "edit \"../kb/x.md\" {\n  delete one text \"x\"\n}\n",
    )?;
    assert!(refused(edit(&w, apply(&w, &[outside], true, false))).is_ok());
    Ok(())
}

#[test]
fn worktree_refusals() -> TestResult {
    let w = world(&[])?;
    let recipes = [recipe(&w, "retry.spl", RETRY)?];
    let why = refused(edit(&w, apply(&w, &recipes, false, false)))?;
    assert!(
        why.contains("records no worktree; pass --worktree DIR"),
        "{why}"
    );
    for dir in [w.base.join("gone"), w.code.join("src")] {
        let mut verb = apply(&w, &recipes, false, false);
        if let EditVerb::Apply { worktree, .. } = &mut verb {
            *worktree = Some(dir);
        }
        let why = refused(edit(&w, verb))?;
        assert!(
            why.contains("missing here") || why.contains("not the top"),
            "{why}"
        );
    }
    Ok(())
}

/// The file a probe changes, and at which step (only the test below sets
/// it).
static MEDDLE: Mutex<Option<(PathBuf, Step)>> = Mutex::new(None);

fn meddle(step: Step) {
    let Ok(mut slot) = MEDDLE.lock() else {
        return;
    };
    if slot.as_ref().is_some_and(|(_, at)| *at == step)
        && let Some((path, _)) = slot.take()
    {
        let _ = std::fs::write(&path, "// changed by someone else\n");
    }
}

#[test]
fn a_file_changed_before_its_rename_refuses_and_restores() -> TestResult {
    for (step, changed, restored) in [
        (Step::Locked, "src/retry.rs", "nothing was written"),
        (
            Step::TempSynced,
            "src/ledger.rs",
            "1 file(s) already replaced were restored",
        ),
    ] {
        let w = world(&[])?;
        let paths = ["src/retry.rs", "src/ledger.rs"];
        let original = snapshot(&w, &paths)?;
        let recipes = [
            recipe(&w, "retry.spl", RETRY)?,
            recipe(&w, "ledger.spl", LEDGER)?,
        ];
        if let Ok(mut slot) = MEDDLE.lock() {
            *slot = Some((w.code.join(changed), step));
        }
        let mut request = w.request(Verb::Edit(apply(&w, &recipes, true, false)));
        request.probe = meddle;
        let why = match writfold_kb::run(&request) {
            Err(KbError::Refused(why)) => why,
            other => {
                return Err(format!("expected a refusal, got {:?}", other.map(|o| o.text)).into());
            }
        };
        assert!(
            why.contains(&format!("{changed} changed since the recipe was replayed")),
            "{why}"
        );
        assert!(why.contains(restored), "{why}");
        let now = snapshot(&w, &paths)?;
        for (i, path) in paths.iter().enumerate() {
            if *path == changed {
                assert_eq!(
                    now[i], b"// changed by someone else\n",
                    "the other edit was lost"
                );
            } else {
                assert_eq!(now[i], original[i], "{path} was left written");
            }
        }
        assert!(receipts(&w).is_empty());
        assert!(
            !w.kb
                .join(".git/writfold/edits")
                .join(format!("{RUN}.json"))
                .exists()
        );
        no_temporaries(&w)?;
    }
    Ok(())
}

#[test]
fn an_edit_in_scope_makes_a_satisfied_gate_stale() -> TestResult {
    let w = world(&[])?;
    let gates = w.code.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    std::fs::write(
        gates.join("unit.toml"),
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n",
    )?;
    let gate = |verb: GateVerb| w.run(&w.request(Verb::Gate(verb)));
    let (text, exit) = gate(GateVerb::Run {
        id: "unit".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    })?;
    assert_eq!(exit, 0, "{text}");
    let status = || {
        gate(GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        })
    };
    let (text, _) = status()?;
    assert!(text.contains("gate unit [satisfied]"), "{text}");
    edit(
        &w,
        apply(&w, &[recipe(&w, "retry.spl", RETRY)?], true, false),
    )?;
    let (text, exit) = status()?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("gate unit [stale]"), "{text}");
    assert!(text.contains("changed in scope: src/retry.rs"), "{text}");
    let _ = NOW;
    Ok(())
}
