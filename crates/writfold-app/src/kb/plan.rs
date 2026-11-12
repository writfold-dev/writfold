//! `writfold kb plan set|status|advance|skip`: arguments only; the run
//! graph is in `writfold_kb::plan`.
use super::args::Common;
use super::writes::get;
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;

pub const USAGE: &str = "usage: plan set <template>|--from FILE [--run ID] [--repo DIR] | plan status [--run ID] [--repo DIR] | plan advance [--run ID] [--repo DIR] | plan skip --reason TEXT --by WHO [--run ID] [--repo DIR]";

/// The plan verb, or why the arguments do not name one.
pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    let run = get(common, "run");
    let repo = get(common, "repo").map(PathBuf::from);
    let from = get(common, "from").map(PathBuf::from);
    let (reason, by) = (get(common, "reason"), get(common, "by"));
    let sub = positional.first().map(String::as_str);
    if from.is_some() && sub != Some("set") {
        return Err("--from belongs to plan set".into());
    }
    if (reason.is_some() || by.is_some()) && sub != Some("skip") {
        return Err("--reason and --by belong to plan skip".into());
    }
    let verb = match positional {
        [s, name] if s == "set" && from.is_none() => PlanVerb::Set {
            template: Some(name.clone()),
            from: None,
            run,
            repo,
        },
        [s] if s == "set" && from.is_some() => PlanVerb::Set {
            template: None,
            from,
            run,
            repo,
        },
        [s] if s == "status" => PlanVerb::Status { run, repo },
        [s] if s == "advance" => PlanVerb::Advance { run, repo },
        [s] if s == "skip" => PlanVerb::Skip {
            reason: reason.ok_or("plan skip needs --reason TEXT")?,
            by: by.ok_or("plan skip needs --by WHO")?,
            run,
            repo,
        },
        _ => return Err(USAGE.into()),
    };
    Ok(Verb::Plan(verb))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::plan::PlanVerb;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn plan_verbs() -> Result<(), String> {
        assert!(matches!(
            verb("plan set change --run r1")?,
            Verb::Plan(PlanVerb::Set { template: Some(ref t), run: Some(_), .. }) if t == "change"
        ));
        assert!(matches!(
            verb("plan set --from plan.toml")?,
            Verb::Plan(PlanVerb::Set {
                template: None,
                from: Some(_),
                ..
            })
        ));
        assert!(matches!(
            verb("plan status")?,
            Verb::Plan(PlanVerb::Status { .. })
        ));
        assert!(matches!(
            verb("plan advance --repo .")?,
            Verb::Plan(PlanVerb::Advance { repo: Some(_), .. })
        ));
        assert!(matches!(
            verb("plan skip --reason later --by me")?,
            Verb::Plan(PlanVerb::Skip { ref reason, .. }) if reason == "later"
        ));
        for bad in [
            "plan",
            "plan set",
            "plan set a b",
            "plan set a --from f",
            "plan status --from f",
            "plan advance --by me",
            "plan skip --reason x",
            "plan skip --by me",
            "plan advance x",
            "plan status --note x",
            "plan sing",
        ] {
            assert!(verb(bad).is_err(), "{bad}");
        }
        Ok(())
    }
}
