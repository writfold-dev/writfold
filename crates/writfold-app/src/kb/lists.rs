//! The listing verbs built in v0.6.0-dev.1 that need more than a switch:
//! `runs --idle AGE` and `delta`; and `resume [<run>] [--here]`.
use super::args::Common;
use writfold_kb::Verb;

/// `runs --idle AGE [--project P] [--plan]`.
pub fn idle(common: &Common) -> Result<Verb, String> {
    let text = common.values.get("idle").cloned().unwrap_or_default();
    if common.switches.iter().any(|s| s == "open") {
        return Err("runs --idle lists open runs already; drop --open".into());
    }
    let age = writfold_kb::verbs::runs::idle_age(&text)
        .ok_or_else(|| format!("--idle takes an age such as 7d or 36h, not {text}"))?;
    Ok(Verb::RunsIdle {
        age,
        age_text: text,
        project: common.project.clone(),
        plan: common.switches.iter().any(|s| s == "plan"),
    })
}

/// `delta --mark NAME | --since-mark NAME | --since REV`: exactly one.
pub fn delta(common: &Common, positional: &[String]) -> Result<Verb, String> {
    use writfold_kb::delta::DeltaVerb;
    if !positional.is_empty() {
        return Err("delta takes no arguments, only one of --mark, --since-mark or --since".into());
    }
    let get = |k: &str| common.values.get(k).cloned();
    match (get("mark"), get("since-mark"), get("since")) {
        (Some(name), None, None) => Ok(Verb::Delta(DeltaVerb::Mark(name))),
        (None, Some(name), None) => Ok(Verb::Delta(DeltaVerb::SinceMark(name))),
        (None, None, Some(rev)) => Ok(Verb::Delta(DeltaVerb::Since(rev))),
        _ => Err("delta takes exactly one of --mark NAME, --since-mark NAME or --since REV".into()),
    }
}

/// `resume [<run>]`, and `resume <run> --here`: make the run this seat's
/// current run (local pointers), then show it.
pub fn resume(here: bool, positional: &[String]) -> Result<Verb, String> {
    match (positional, here) {
        ([], false) => Ok(Verb::Resume { run: None }),
        ([run], false) => Ok(Verb::Resume {
            run: Some(run.clone()),
        }),
        ([run], true) => Ok(Verb::Attach { run: run.clone() }),
        (_, true) => Err("resume --here takes one run".into()),
        _ => Err("resume: too many arguments".into()),
    }
}
