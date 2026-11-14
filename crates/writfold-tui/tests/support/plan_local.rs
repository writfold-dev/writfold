use super::super::{NOW, RUN, TestResult, World, write};
use serde_json::json;
use std::path::Path;
use writfold_kb::{draft, proposal};

pub fn guards(kb: &Path) -> TestResult {
    write(
        &kb.join(".writfold/guards/commit-needs-gates.toml"),
        concat!(
            "schema = \"writfold-guard-rule/1\"\nid = \"commit-needs-gates\"\nevent = \"pre-tool-use\"\n",
            "commands = [\"git commit\"]\nrequire = [\"gates.met\"]\naction = \"deny\"\n"
        ),
    )?;
    write(
        &kb.join(".writfold/guards/stop-reflect.toml"),
        concat!(
            "schema = \"writfold-guard-rule/1\"\nid = \"stop-reflect\"\nevent = \"stop\"\n",
            "require = [\"step.met\"]\n"
        ),
    )
}

pub fn inbox(w: &World) -> TestResult {
    let ws = writfold_kb::workspace::discover(Some(&w.kb), &w.kb)?;
    let state = proposal::state_of(&ws).ok_or("local state")?;
    write(&state.join("current-run"), &format!("{RUN}\n"))?;
    let note = json!({"schema": draft::SCHEMA, "session": "s-progress", "run": RUN,
        "updated": writfold_kb::time::stamp(NOW - 90),
        "last_message": "Review started; lint went stale after the rename."});
    draft::save(&state, "s-progress", note.as_object().ok_or("draft")?)?;
    for (kind, args, run) in [
        (
            "ticket",
            json!({"id": "HBR-210", "title": "Lint gate scope misses the command module"}),
            Some(RUN.into()),
        ),
        (
            "link",
            json!({"from": RUN, "to": super::DECISION, "type": "mentions"}),
            None,
        ),
    ] {
        let args = args.as_object().ok_or("proposal arguments")?.clone();
        let item = proposal::Proposal {
            id: proposal::digest(kind, &args),
            kind: kind.into(),
            args,
            session: Some("s-progress".into()),
            run,
            created: writfold_kb::time::stamp(NOW - 60),
        };
        proposal::save(&state, &item, writfold_kb::write::no_probe)?;
    }
    Ok(())
}
