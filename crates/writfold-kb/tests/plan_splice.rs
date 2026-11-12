//! Splicing: `plan use` puts a template's steps before the current step,
//! as its children, and agrees with the kernel's `splice`; a repeat (also
//! from a child step, as a skill reloaded after compaction would) writes
//! nothing and exits 0; `--again` adds the next ordinal; `use=` includes
//! expand at splice time with their closure snapshotted; a cycle and a
//! nesting deeper than four levels are refused; an edited template shows
//! `!template-moved` and changes no step; the fold stays a function of the
//! set of event files.
mod support;

use std::path::Path;
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::plan::{PlanVerb, fold};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";

pub fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// A template `name` with these steps.
pub fn template(w: &World, name: &str, steps: &[&str]) -> TestResult {
    let list: Vec<String> = steps.iter().map(|s| format!("\"{s}\"")).collect();
    let text = format!(
        "schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{}]\n",
        list.join(", ")
    );
    put(&w.kb.join(format!(".writfold/plans/{name}.toml")), &text)
}

pub fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let re = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let marked = text.replace(RUN, "<RUN>");
    let stable = re.replace_all(&marked, "-$1-XXXXXX");
    let actual = format!("{}# exit={exit}\n", stable.replace("<RUN>", RUN));
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

pub fn plan(w: &World, verb: &str, name: &str) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let verb = match verb {
        "set" => PlanVerb::Set {
            template: Some(name.into()),
            from: None,
            run,
            repo,
        },
        "use" | "again" => PlanVerb::Use {
            template: name.into(),
            again: verb == "again",
            run,
            repo,
        },
        "status" => PlanVerb::Status { run, repo },
        "skip" => PlanVerb::Skip {
            reason: name.into(),
            by: "owner".into(),
            run,
            repo,
        },
        _ => PlanVerb::Advance { run, repo },
    };
    w.run(&w.request(Verb::Plan(verb)))
}

pub fn events(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

pub fn graph(w: &World) -> fold::Graph {
    fold::read(&w.kb, &format!("runs/{RUN}"), RUN)
}

fn states(g: &fold::Graph) -> Vec<(bool, bool)> {
    g.nodes().iter().map(|n| (n.done, n.skipped)).collect()
}

/// `plan use` with no plan is a root splice; under a step it inserts
/// before the current step exactly as the kernel's `splice` does; a
/// repeat is a no-op; `--again` takes the next ordinal.
#[test]
fn use_splices_before_the_current_step_once() -> TestResult {
    let w = world(&[])?;
    template(&w, "fix", &["reproduce:", "implement:", "deliver:"])?;
    template(&w, "review", &["read-diff:", "findings:"])?;
    let (text, exit) = plan(&w, "use", "fix")?;
    assert!(exit == 0 && text.contains("had no plan"), "{text}");
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    let before = graph(&w);
    let at = before.current().ok_or("a current step")?;
    let (text, exit) = plan(&w, "use", "review")?;
    golden("plan-use.txt", &text, exit)?;
    let after = graph(&w);
    let kernel = writfold_core::plan::splice(&before.nodes(), at, 2);
    let kernel: Vec<(bool, bool)> = kernel.iter().map(|n| (n.done, n.skipped)).collect();
    assert_eq!(states(&after), kernel, "the fold agrees with the kernel");
    let paths: Vec<&str> = after.steps.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "fix#1/reproduce",
            "fix#1/implement/review#1/read-diff",
            "fix#1/implement/review#1/findings",
            "fix#1/implement",
            "fix#1/deliver"
        ]
    );
    // A repeat from the child step the splice made current: nothing written.
    let written = events(&w);
    let (text, exit) = plan(&w, "use", "review")?;
    golden("plan-use-repeat.txt", &text, exit)?;
    assert_eq!(events(&w), written);
    // Walk the children; at implement again, a repeat is still a no-op,
    // and --again splices review#2.
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    let written = events(&w);
    let (text, exit) = plan(&w, "use", "review")?;
    assert!(exit == 0 && text.contains("already spliced"), "{text}");
    assert_eq!(events(&w), written);
    let (text, exit) = plan(&w, "again", "review")?;
    assert!(
        exit == 0 && text.contains("step fix#1/implement/review#2/read-diff [current]"),
        "{text}"
    );
    assert_eq!(events(&w).len(), written.len() + 1);
    Ok(())
}

/// `use=` includes expand at splice time, their snapshots kept; a
/// `plan use` of an included template is already spliced.
#[test]
fn includes_expand_with_their_closure() -> TestResult {
    let w = world(&[])?;
    template(&w, "ship", &["build:", "check: use=review", "release:"])?;
    template(&w, "review", &["read-diff: use=notes", "findings:"])?;
    template(&w, "notes", &["note:"])?;
    let (text, exit) = plan(&w, "set", "ship")?;
    golden("plan-set-includes.txt", &text, exit)?;
    let snapshots = std::fs::read_dir(w.kb.join(format!("runs/{RUN}/plan/templates")))?;
    assert_eq!(snapshots.count(), 3, "one snapshot per template");
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    let (text, exit) = plan(&w, "use", "review")?;
    assert!(
        exit == 0 && text.contains("review is already spliced under ship#1/check"),
        "{text}"
    );
    let (text, exit) = plan(&w, "use", "notes")?;
    assert!(exit == 0 && text.contains("already spliced"), "{text}");
    // Editing an included template moves it; the run's steps are unchanged.
    let steps =
        |g: &fold::Graph| -> Vec<String> { g.steps.iter().map(|s| s.path.clone()).collect() };
    let before = steps(&graph(&w));
    template(&w, "notes", &["note:", "more:"])?;
    assert_eq!(steps(&graph(&w)), before);
    let (text, _) = plan(&w, "status", "")?;
    assert!(
        text.contains("step ship#1/check/review#1/read-diff/notes#1/note [current]")
            && text.contains("!template-moved")
            && text.contains("template notes moved"),
        "{text}"
    );
    // Only the moved template's steps carry the flag.
    let findings = text
        .lines()
        .find(|l| l.contains("review#1/findings "))
        .ok_or("findings")?;
    assert!(!findings.contains("template-moved"), "{text}");
    Ok(())
}

/// A cycle among includes and a nesting deeper than four levels (static
/// or by splicing under a deep step) are refused, writing nothing.
#[test]
fn cycles_and_depth_are_refused() -> TestResult {
    let w = world(&[])?;
    template(&w, "loop-a", &["a: use=loop-b"])?;
    template(&w, "loop-b", &["b: use=loop-a"])?;
    let refused = plan(&w, "set", "loop-a");
    assert!(refused.is_err_and(|e| {
        e.to_string()
            .contains("loop-a includes itself: loop-a → loop-b → loop-a")
    }),);
    for (i, name) in ["d1", "d2", "d3", "d4"].iter().enumerate() {
        template(&w, name, &[&format!("s{i}: use=d{}", i + 2)])?;
    }
    template(&w, "d5", &["s4:"])?;
    let refused = plan(&w, "set", "d1");
    assert!(refused.is_err_and(|e| e.to_string().contains("nests deeper than 4 levels")));
    assert!(events(&w).is_empty(), "{:?}", events(&w));
    // d2 → d3 → d4 → d5 is four levels: admitted.
    assert_eq!(plan(&w, "set", "d2")?.1, 0);
    let g = graph(&w);
    let current = g.current().map(|i| g.steps[i].path.clone());
    assert_eq!(
        current.as_deref(),
        Some("d2#1/s1/d3#1/s2/d4#1/s3/d5#1/s4"),
        "{:?}",
        g.steps
    );
    // Splicing anything under a step four levels deep is refused.
    template(&w, "leaf", &["y:"])?;
    let written = events(&w);
    let refused = plan(&w, "use", "leaf");
    assert!(refused.is_err_and(|e| e.to_string().contains("would nest deeper than 4 levels")),);
    assert_eq!(events(&w), written);
    Ok(())
}

/// A template edited after it was spliced: `!template-moved` on its steps
/// in `plan status` and on `resume`'s plan line; every step, state and
/// event is unchanged.
#[test]
fn an_edited_template_is_flagged_and_changes_nothing() -> TestResult {
    let w = world(&[])?;
    template(&w, "fix", &["reproduce:", "implement:"])?;
    plan(&w, "set", "fix")?;
    let (before, _) = plan(&w, "status", "")?;
    let written = events(&w);
    template(&w, "fix", &["reproduce:", "implement:", "deliver:"])?;
    let (text, exit) = plan(&w, "status", "")?;
    golden("plan-status-moved.txt", &text, exit)?;
    assert_eq!(events(&w), written);
    let unflagged: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("step "))
        .map(|l| l.replace(" !template-moved", ""))
        .collect();
    let expected: Vec<&str> = before.lines().filter(|l| l.starts_with("step ")).collect();
    assert_eq!(unflagged, expected);
    let mut resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    resume.cwd = w.code.clone();
    let (text, _) = w.run(&resume)?;
    let line = text
        .lines()
        .find(|l| l.starts_with("plan "))
        .ok_or("a plan line")?;
    assert!(line.ends_with("!template-moved"), "{text}");
    // Moving it back clears the flag.
    template(&w, "fix", &["reproduce:", "implement:"])?;
    let (text, _) = plan(&w, "status", "")?;
    assert!(!text.contains("template-moved"), "{text}");
    Ok(())
}

/// With nested splices, an include, a skip and `--again`, every rotation
/// and reversal of the listing folds the same, and a clone answers alike.
#[test]
fn the_fold_with_splices_depends_only_on_the_set_of_files() -> TestResult {
    let w = world(&[])?;
    template(
        &w,
        "fix",
        &["reproduce:", "implement: use=notes", "deliver:"],
    )?;
    template(&w, "review", &["read-diff:", "findings:"])?;
    template(&w, "notes", &["note:"])?;
    plan(&w, "set", "fix")?;
    plan(&w, "use", "review")?;
    plan(&w, "skip", "not needed")?;
    plan(&w, "again", "review")?;
    plan(&w, "advance", "")?;
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let mut files: Vec<(String, String)> = Vec::new();
    for name in events(&w).into_iter().filter(|n| n.ends_with(".json")) {
        files.push((name.clone(), std::fs::read_to_string(dir.join(&name))?));
    }
    assert!(files.len() >= 5, "{files:?}");
    let snapshot = |digest: &str| {
        let hex = digest.strip_prefix("sha256:")?;
        std::fs::read(dir.join("templates").join(format!("{hex}.toml"))).ok()
    };
    let shape = |files: Vec<(String, String)>| {
        let g = fold::fold(RUN, "x", files, &snapshot);
        let steps: Vec<String> = g
            .steps
            .iter()
            .map(|s| {
                format!(
                    "{} {} {} {:?}",
                    s.path,
                    s.done.is_some(),
                    s.skipped.is_some(),
                    s.begin
                )
            })
            .collect();
        let current = g.current();
        (steps, g.invalid, current, g.uses)
    };
    let expected = shape(files.clone());
    assert!(expected.1.is_empty(), "{:?}", expected.1);
    for turn in 0..files.len() {
        let mut turned = files.clone();
        turned.rotate_left(turn);
        assert_eq!(shape(turned.clone()), expected);
        turned.reverse();
        assert_eq!(shape(turned), expected);
    }
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "plan"])?;
    let clone = w.base.join("clone");
    git(
        &w.base,
        &["clone", "-q", &w.kb.display().to_string(), "clone"],
    )?;
    std::fs::create_dir_all(clone.join(".git/writfold"))?;
    std::fs::copy(
        w.kb.join(".git/writfold/repos"),
        clone.join(".git/writfold/repos"),
    )?;
    let rows = |kb: &Path| -> TestResult<Vec<String>> {
        let mut request = w.request(Verb::Plan(PlanVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }));
        request.kb = Some(kb.to_path_buf());
        let out = writfold_kb::run(&request)?;
        Ok(out
            .text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect())
    };
    assert_eq!(rows(&clone)?, rows(&w.kb)?);
    Ok(())
}
