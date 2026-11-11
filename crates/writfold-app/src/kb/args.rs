//! `writfold kb` arguments into one typed request.
use std::path::PathBuf;
use writfold_kb::{DEFAULT_BUDGET, MAX_BUDGET, Request, Verb};

#[derive(Default)]
struct Common {
    kb: Option<PathBuf>,
    budget: Option<usize>,
    cursor: Option<String>,
    json: bool,
    depth: Option<usize>,
    all: bool,
    offline: bool,
}

fn number(flag: &str, value: Option<&String>, low: usize, high: usize) -> Result<usize, String> {
    let value = value.ok_or(format!("{flag} needs a value"))?;
    match value.parse::<usize>() {
        Ok(n) if (low..=high).contains(&n) => Ok(n),
        _ => Err(format!("{flag} must be an integer from {low} to {high}")),
    }
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
            "--kb" => {
                common.kb = Some(PathBuf::from(items.next().ok_or("--kb needs a directory")?))
            }
            "--budget-tokens" => common.budget = Some(number(arg, items.next(), 1, MAX_BUDGET)?),
            "--cursor" => {
                common.cursor = Some(items.next().ok_or("--cursor needs a value")?.clone())
            }
            "--depth" => common.depth = Some(number(arg, items.next(), 1, 8)?),
            "--json" => common.json = true,
            "--all" => common.all = true,
            "--offline" => common.offline = true,
            "--at" => return Err("--at COMMIT is not built in v1 (planned for M4)".into()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok((common, positional))
}

fn why_sections() -> Vec<String> {
    std::env::var("WRITFOLD_KB_WHY_SECTIONS")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
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
    let one = |what: &str| -> Result<String, String> {
        match positional.as_slice() {
            [single] => Ok(single.clone()),
            _ => Err(format!("{verb} takes exactly one {what}")),
        }
    };
    let optional = || -> Result<Option<String>, String> {
        match positional.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(single.clone())),
            _ => Err(format!("{verb} takes at most one argument")),
        }
    };
    let allowed: &[&str] = match verb.as_str() {
        "brief" => &["all"],
        "context" => &["depth"],
        "stale" => &["offline"],
        _ => &[],
    };
    for (flag, set) in [
        ("all", common.all),
        ("depth", common.depth.is_some()),
        ("offline", common.offline),
    ] {
        if set && !allowed.contains(&flag) {
            return Err(format!("--{flag} does not apply to {verb}"));
        }
    }
    let verb = match verb.as_str() {
        "brief" => Verb::Brief {
            project: optional()?,
            all: common.all,
        },
        "context" => Verb::Context {
            target: one("target")?,
            depth: common.depth.unwrap_or(2),
        },
        "why" if !positional.is_empty() => Verb::Why {
            text: positional.join(" "),
            sections: why_sections(),
        },
        "why" => return Err("why takes an ID or some text".into()),
        "stale" => Verb::Stale {
            scope: optional()?,
            offline: common.offline,
        },
        "index" => match one("subcommand (status or rebuild)")?.as_str() {
            "status" => Verb::IndexStatus,
            "rebuild" => Verb::IndexRebuild,
            other => return Err(format!("unknown index subcommand {other}")),
        },
        other => return Err(format!("unknown verb {other}")),
    };
    if matches!(
        &verb,
        Verb::Brief {
            project: Some(_),
            all: true
        }
    ) {
        return Err("brief takes a project or --all, not both".into());
    }
    Ok(Some(Request {
        verb,
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
        assert!(
            parse(&args("index rebuild"), cwd.clone())
                .is_ok_and(|r| r.is_some_and(|r| matches!(r.verb, Verb::IndexRebuild)))
        );
        assert!(parse(&args("help"), cwd).is_ok_and(|r| r.is_none()));
        Ok(())
    }
}
