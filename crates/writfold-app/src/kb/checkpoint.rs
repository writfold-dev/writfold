//! `checkpoint` arguments: `--note TEXT` or `--from-draft SESSION`, with
//! `--run ID` or `--new-run --project P --task T [--mode M]`, and any
//! number of `--artifact <store>@<rev>:<path>`.
use super::args::Common;
use super::writes::{get, need, switch};
use writfold_kb::write::Write;
use writfold_kb::write::run::{Checkpoint, FromDraft, NewRun};

/// Value flags that may be given more than once; their values are kept in
/// order, separated by a line break.
pub const REPEATS: [&str; 1] = ["artifact"];

/// Record `value` for `flag`: once, or appended for a repeatable flag.
pub fn put(values: &mut std::collections::BTreeMap<String, String>, flag: &str, value: String) {
    match values.get_mut(flag) {
        Some(old) if REPEATS.contains(&flag) => {
            old.push('\n');
            old.push_str(&value);
        }
        _ => {
            values.insert(flag.to_owned(), value);
        }
    }
}

/// The values of a repeatable flag, in order.
pub fn all(common: &Common, flag: &str) -> Vec<String> {
    common
        .values
        .get(flag)
        .map(|v| v.split('\n').map(str::to_owned).collect())
        .unwrap_or_default()
}

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
    let artifacts = all(common, "artifact");
    match (get(common, "note"), get(common, "from-draft")) {
        (Some(_), Some(_)) => Err("checkpoint takes --note or --from-draft, not both".into()),
        (None, None) => Err("checkpoint needs --note TEXT or --from-draft SESSION".into()),
        (Some(note), None) => Ok(Write::Checkpoint(Checkpoint {
            run,
            new_run,
            note,
            artifacts,
        })),
        (None, Some(session)) => Ok(Write::FromDraft(FromDraft {
            session,
            run,
            new_run,
            artifacts,
        })),
    }
}

#[cfg(test)]
mod tests {
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        super::super::args::verb_of(&args)
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
        let two = verb("checkpoint --note n --artifact s@a:x --artifact s@b:y")?;
        assert!(matches!(two, Verb::Write(Write::Checkpoint(c))
            if c.artifacts == ["s@a:x", "s@b:y"]));
        assert!(verb("checkpoint --note n --note m").is_err());
        Ok(())
    }
}
