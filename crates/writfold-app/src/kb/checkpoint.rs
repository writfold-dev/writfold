//! `checkpoint` arguments: `--note TEXT` or `--from-draft SESSION`, with
//! `--run ID` or `--new-run --project P --task T [--mode M]`.
use super::args::Common;
use super::writes::{get, need, switch};
use writfold_kb::write::Write;
use writfold_kb::write::run::{Checkpoint, FromDraft, NewRun};

pub fn checkpoint(common: &Common) -> Result<Write, String> {
    let new_run = if switch(common, "new-run") {
        if common.values.contains_key("run") {
            return Err("checkpoint takes --run or --new-run, not both".into());
        }
        Some(NewRun {
            project: common.project.clone().ok_or("--new-run needs --project")?,
            task: need(common, "--new-run", "task")?,
            mode: get(common, "mode"),
        })
    } else {
        if common.project.is_some() || common.values.contains_key("task") {
            return Err("--project and --task go with --new-run".into());
        }
        None
    };
    let run = get(common, "run");
    match (get(common, "note"), get(common, "from-draft")) {
        (Some(_), Some(_)) => Err("checkpoint takes --note or --from-draft, not both".into()),
        (None, None) => Err("checkpoint needs --note TEXT or --from-draft SESSION".into()),
        (Some(note), None) => Ok(Write::Checkpoint(Checkpoint { run, new_run, note })),
        (None, Some(session)) => Ok(Write::FromDraft(FromDraft {
            session,
            run,
            new_run,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn note_or_draft() -> Result<(), String> {
        let draft = verb("checkpoint --from-draft s-1 --run r")?;
        assert!(matches!(draft, Verb::Write(Write::FromDraft(f))
            if f.session == "s-1" && f.run.as_deref() == Some("r")));
        assert!(verb("checkpoint --from-draft s --note n").is_err());
        assert!(verb("checkpoint").is_err());
        assert!(matches!(
            verb("checkpoint --note n")?,
            Verb::Write(Write::Checkpoint(_))
        ));
        Ok(())
    }
}
