//! `writfold kb` arguments into one typed request.
use super::verbs;
use std::path::PathBuf;
use writfold_kb::{DEFAULT_BUDGET, MAX_BUDGET, Request, Verb};

/// Every flag, parsed once; `verbs` checks which apply.
#[derive(Default)]
pub struct Common {
    pub kb: Option<PathBuf>,
    pub budget: Option<usize>,
    pub cursor: Option<String>,
    pub json: bool,
    pub depth: Option<usize>,
    pub all: bool,
    pub offline: bool,
    pub kinds: Vec<String>,
    pub project: Option<String>,
    pub signatures: bool,
    pub direction: Option<String>,
    pub include: Option<String>,
}

fn number(flag: &str, value: Option<&String>, low: usize, high: usize) -> Result<usize, String> {
    let value = value.ok_or(format!("{flag} needs a value"))?;
    match value.parse::<usize>() {
        Ok(n) if (low..=high).contains(&n) => Ok(n),
        _ => Err(format!("{flag} must be an integer from {low} to {high}")),
    }
}

fn value(flag: &str, value: Option<&String>) -> Result<String, String> {
    value.cloned().ok_or(format!("{flag} needs a value"))
}

/// Split flags from positionals. Every flag may appear once.
fn split(args: &[String]) -> Result<(Common, Vec<String>), String> {
    let mut common = Common::default();
    let mut positional = Vec::new();
    let mut seen = Vec::new();
    let mut items = args.iter();
    while let Some(arg) = items.next() {
        if !arg.starts_with("--") {
            positional.push(arg.clone());
            continue;
        }
        if seen.contains(arg) {
            return Err(format!("{arg} given twice"));
        }
        seen.push(arg.clone());
        match arg.as_str() {
            "--kb" => common.kb = Some(PathBuf::from(value(arg, items.next())?)),
            "--budget-tokens" => common.budget = Some(number(arg, items.next(), 1, MAX_BUDGET)?),
            "--cursor" => common.cursor = Some(value(arg, items.next())?),
            "--depth" => common.depth = Some(number(arg, items.next(), 1, 8)?),
            "--json" => common.json = true,
            "--all" => common.all = true,
            "--offline" => common.offline = true,
            "--signatures" => common.signatures = true,
            "--kind" => {
                common.kinds = value(arg, items.next())?
                    .split(',')
                    .map(|k| k.trim().to_owned())
                    .filter(|k| !k.is_empty())
                    .collect();
            }
            "--project" => common.project = Some(value(arg, items.next())?),
            "--direction" => common.direction = Some(value(arg, items.next())?),
            "--include" => common.include = Some(value(arg, items.next())?),
            "--at" => return Err("--at COMMIT is not built in v1 (planned for M4)".into()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok((common, positional))
}

/// The flags each verb accepts beyond the shared ones.
fn allowed(verb: &str) -> &'static [&'static str] {
    match verb {
        "brief" => &["all"],
        "context" => &["depth"],
        "stale" => &["offline"],
        "find" => &["kind", "project", "signatures"],
        "match" => &["kind", "project"],
        "impact" => &["depth", "direction", "include"],
        _ => &[],
    }
}

fn check_flags(verb: &str, common: &Common) -> Result<(), String> {
    let allowed = allowed(verb);
    for (flag, set) in [
        ("all", common.all),
        ("depth", common.depth.is_some()),
        ("offline", common.offline),
        ("kind", !common.kinds.is_empty()),
        ("project", common.project.is_some()),
        ("signatures", common.signatures),
        ("direction", common.direction.is_some()),
        ("include", common.include.is_some()),
    ] {
        if set && !allowed.contains(&flag) {
            return Err(format!("--{flag} does not apply to {verb}"));
        }
    }
    Ok(())
}

/// `None` for `help`; an error message for anything unusable.
pub fn parse(args: &[String], cwd: PathBuf) -> Result<Option<Request>, String> {
    let Some((verb, rest)) = args.split_first() else {
        return Err("missing verb".into());
    };
    if matches!(verb.as_str(), "help" | "--help" | "-h") {
        return Ok(None);
    }
    let (common, positional) = split(rest)?;
    check_flags(verb, &common)?;
    let built: Verb = verbs::verb(verb, &common, &positional)?;
    Ok(Some(Request {
        verb: built,
        kb: common.kb,
        cwd,
        budget: common.budget.unwrap_or(DEFAULT_BUDGET),
        cursor: common.cursor,
        json: common.json,
        now: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;

    fn args(text: &str) -> Vec<String> {
        text.split(' ').map(str::to_owned).collect()
    }

    #[test]
    fn verbs_and_flags() -> Result<(), String> {
        let cwd = PathBuf::from("/");
        let request = parse(
            &args("why batch ledger writes --budget-tokens 200 --json"),
            cwd.clone(),
        )?
        .ok_or("a request")?;
        assert!(matches!(&request.verb, Verb::Why { text, .. } if text == "batch ledger writes"));
        assert_eq!((request.budget, request.json), (200, true));
        assert!(parse(&args("context a b"), cwd.clone()).is_err());
        assert!(parse(&args("context a --offline"), cwd.clone()).is_err());
        assert!(parse(&args("brief x --all"), cwd.clone()).is_err());
        assert!(parse(&args("brief --budget-tokens 0"), cwd.clone()).is_err());
        assert!(parse(&args("stale --at abc"), cwd.clone()).is_err());
        assert!(parse(&args("impact x --direction sideways"), cwd.clone()).is_err());
        assert!(parse(&args("impact x --include everything"), cwd.clone()).is_err());
        assert!(parse(&args("status x --kind ticket"), cwd.clone()).is_err());
        let found = parse(&args("match fsync took --kind signature"), cwd.clone())?;
        assert!(found.is_some_and(|r| matches!(
            r.verb,
            Verb::Find { signatures: true, ref text, .. } if text == "fsync took"
        )));
        let impact = parse(
            &args("impact harbor-api --depth 3 --direction in --include inferred"),
            cwd.clone(),
        )?;
        assert!(impact.is_some_and(|r| matches!(
            r.verb,
            Verb::Impact {
                depth: 3,
                inferred: true,
                ..
            }
        )));
        assert!(
            parse(&args("index verify"), cwd.clone())
                .is_ok_and(|r| r.is_some_and(|r| matches!(r.verb, Verb::IndexVerify)))
        );
        assert!(parse(&args("help"), cwd).is_ok_and(|r| r.is_none()));
        Ok(())
    }
}
