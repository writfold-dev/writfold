//! Local inbox and subject edits refresh without a manual reload.
mod support;

use serde_json::json;
use support::{NOW, RUN, TestResult, World, git, world};
use writfold_kb::gate::GateVerb;
use writfold_kb::proposal::{self, Proposal};
use writfold_kb::workspace::discover;
use writfold_kb::{Verb, write};
use writfold_tui::app::App;
use writfold_tui::source::{Source, field};
use writfold_tui::style::Environment;

#[test]
fn a_local_proposal_refreshes_the_inbox_and_keeps_selection() -> TestResult {
    let w = world("local-refresh")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(writfold_tui::app::View::Resume);
    assert!(!app.poll());
    let ws = discover(Some(&w.kb), &w.kb)?;
    let state = proposal::state_of(&ws).ok_or("state")?;
    let args = json!({"ticket":"HBR-210", "title":"Review the scope"});
    let args = args.as_object().ok_or("arguments")?.clone();
    let item = Proposal {
        id: proposal::digest("ticket", &args),
        kind: "ticket".into(),
        args,
        session: Some("watch".into()),
        run: Some(RUN.into()),
        created: writfold_kb::time::stamp(NOW),
    };
    proposal::save(&state, &item, write::no_probe)?;
    assert!(app.poll());
    assert_eq!(app.run().as_deref(), Some(RUN));
    assert!(
        app.detail()
            .ok_or("resume")?
            .rows
            .iter()
            .any(|r| field(r, "id") == item.id)
    );
    assert!(!app.poll());
    Ok(())
}

fn subject_source(w: &World) -> TestResult<Source> {
    let repo = w.base.join("subject");
    std::fs::create_dir(&repo)?;
    std::fs::write(repo.join("tracked.txt"), "checked\n")?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-q", "-m", "subject"])?;
    std::fs::write(
        w.kb.join(".writfold/gates/subject.toml"),
        "schema = \"writfold-gate/1\"\nid = \"subject\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"tracked.txt\"]\n",
    )?;
    w.verb(
        Verb::Gate(GateVerb::Run {
            id: "subject".into(),
            run: Some(RUN.into()),
            repo: Some(repo.clone()),
        }),
        false,
    )?;
    Ok(Source::located(None, w.kb.clone(), Some(repo), Some(NOW))?)
}

fn standing(app: &App) -> TestResult<&str> {
    app.gate_status()
        .ok_or("gates")?
        .rows
        .iter()
        .find(|r| field(r, "id") == "subject")
        .map(|r| field(r, "status"))
        .ok_or_else(|| "subject gate".into())
}

#[test]
fn subject_edit_turns_a_gate_stale_on_the_fifth_tick() -> TestResult {
    let w = world("subject-refresh")?;
    let source = subject_source(&w)?;
    let repo = source.cwd.clone();
    assert_eq!(source.kb.as_deref(), Some(w.kb.as_path()));
    let mut app = App::new(source, Some(RUN));
    assert_eq!(standing(&app)?, "satisfied");
    std::fs::write(repo.join("tracked.txt"), "edited since the receipt\n")?;
    for _ in 0..4 {
        assert!(!app.poll());
    }
    assert_eq!(standing(&app)?, "satisfied");
    assert!(app.poll());
    assert_eq!(standing(&app)?, "stale");
    assert_eq!(app.run().as_deref(), Some(RUN));
    assert!(!app.poll());
    Ok(())
}

#[test]
fn repo_is_a_subject_override_not_a_kb_discovery_override() -> TestResult {
    let w = world("repo-option")?;
    let source = subject_source(&w)?;
    let args = vec![
        "--repo".into(),
        source.cwd.display().to_string(),
        "--once".into(),
    ];
    let text = writfold_tui::main(&args, w.kb.clone(), false, Environment::default())?
        .stdout
        .ok_or("frame")?;
    assert!(text.contains("subject=subject@"), "{text}");
    assert!(text.contains("gate subject [satisfied]"), "{text}");
    for dir in [w.base.clone(), w.base.join("missing")] {
        let error = Source::located(None, w.kb.clone(), Some(dir.clone()), None)
            .err()
            .ok_or("accepted unusable subject")?;
        assert!(
            error.contains("--repo") && error.contains(&dir.display().to_string()),
            "{error}"
        );
    }
    Ok(())
}
