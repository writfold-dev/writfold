//! `writfold kb` arguments into one typed request.
use super::{cli, verbs, writes};
use std::collections::BTreeMap;
use std::path::PathBuf;
use writfold_kb::{DEFAULT_BUDGET, Request, Verb};

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
    pub cached: bool,
    pub record: bool,
    pub kinds: Vec<String>,
    pub project: Option<String>,
    pub signatures: bool,
    pub direction: Option<String>,
    pub include: Option<String>,
    pub missing: bool,
    pub limit: Option<usize>,
    /// `--color auto|always|never`: how diagnostics are drawn.
    pub color: Option<String>,
    /// The write verbs' value flags (`--note`, `--title` …) and switches.
    pub values: BTreeMap<String, String>,
    pub switches: Vec<String>,
}

/// `value` as an integer from `low` to `high`, or the message for `flag`.
pub(crate) fn number(flag: &str, value: &str, low: usize, high: usize) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(n) if (low..=high).contains(&n) => Ok(n),
        _ => Err(format!("{flag} must be an integer from {low} to {high}")),
    }
}

/// The flags each verb accepts beyond the shared ones.
pub(crate) fn allowed(verb: &str) -> &'static [&'static str] {
    match verb {
        "brief" | "deliver" => &["all"],
        "context" => &["depth"],
        "stale" => &["offline", "cached", "record"],
        "repos" => &["missing"],
        "find" => &["kind", "project", "signatures", "limit"],
        "match" => &["kind", "project"],
        "cite" => &["kind"],
        "impact" => &["depth", "direction", "include"],
        "runs" | "checkpoint" | "record" | "propose" => &["project"],
        _ => &[],
    }
}

fn check_flags(verb: &str, common: &Common) -> Result<(), String> {
    let allowed = allowed(verb);
    for (flag, set) in [
        ("all", common.all),
        ("depth", common.depth.is_some()),
        ("offline", common.offline),
        ("cached", common.cached),
        ("record", common.record),
        ("kind", !common.kinds.is_empty()),
        ("project", common.project.is_some()),
        ("signatures", common.signatures),
        ("direction", common.direction.is_some()),
        ("include", common.include.is_some()),
        ("missing", common.missing),
        ("limit", common.limit.is_some()),
    ] {
        if set && !allowed.contains(&flag) {
            return Err(format!("--{flag} does not apply to {verb}"));
        }
    }
    let extra = writes::allowed(verb);
    for flag in common.values.keys().chain(common.switches.iter()) {
        if !extra.contains(&flag.as_str()) {
            return Err(format!("--{flag} does not apply to {verb}"));
        }
    }
    Ok(())
}

/// What the line asks for: a request, or text clap rendered (help on
/// stdout, exit 0; a usage error on stderr, exit 1).
pub enum Parsed {
    Request(Box<Request>),
    Help(clap::builder::StyledStr),
    Usage(String),
    Refused(clap::Error),
}

/// Parse `writfold kb <args>`: clap splits the line into flags and
/// positionals ([`cli`]); the verb modules build the typed request.
pub fn parse(args: &[String], cwd: PathBuf) -> Parsed {
    let line = std::iter::once("kb".to_owned()).chain(args.iter().cloned());
    let matches = match cli::command().try_get_matches_from(line) {
        Ok(matches) => matches,
        Err(error) => {
            return match error.kind() {
                clap::error::ErrorKind::DisplayHelp => Parsed::Help(error.render()),
                _ => Parsed::Refused(error),
            };
        }
    };
    let Some((verb, sub)) = matches.subcommand() else {
        return Parsed::Usage("missing verb".into());
    };
    match request(verb, sub, cwd) {
        Ok(request) => Parsed::Request(Box::new(request)),
        Err(message) => Parsed::Usage(message),
    }
}

fn request(verb: &str, sub: &clap::ArgMatches, cwd: PathBuf) -> Result<Request, String> {
    let (common, positional) = cli::common(verb, sub)?;
    check_flags(verb, &common)?;
    if common.cached && common.offline {
        return Err("stale takes --offline or --cached, not both".into());
    }
    if common.record && (common.offline || common.cached) {
        return Err("--record needs the checkers to be asked; it cannot be combined with --offline or --cached".into());
    }
    let built: Verb = verbs::verb(verb, &common, &positional, &cwd)?;
    Ok(Request {
        verb: built,
        kb: common.kb,
        cwd,
        budget: common.budget.unwrap_or(DEFAULT_BUDGET),
        cursor: common.cursor,
        json: common.json,
        now: None,
        probe: writfold_kb::write::no_probe,
    })
}

/// The verb `args` parse to, or the refusal's message (module tests).
#[cfg(test)]
pub fn verb_of(args: &[String]) -> Result<Verb, String> {
    match parse(args, PathBuf::from("/")) {
        Parsed::Request(request) => Ok(request.verb),
        Parsed::Usage(message) => Err(message),
        Parsed::Refused(error) => Err(error.to_string()),
        Parsed::Help(_) => Err("help".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Parsed, parse};
    use std::path::PathBuf;
    use writfold_kb::{Request, Verb};

    /// The request `text` parses to, or the message of its refusal.
    fn req(text: &str) -> Result<Request, String> {
        let args: Vec<String> = text.split(' ').map(str::to_owned).collect();
        match parse(&args, PathBuf::from("/")) {
            Parsed::Request(request) => Ok(*request),
            Parsed::Usage(message) => Err(message),
            Parsed::Refused(error) => Err(error.to_string()),
            Parsed::Help(_) => Err("help".into()),
        }
    }

    #[test]
    fn verbs_and_flags() -> Result<(), String> {
        let request = req("why batch ledger writes --budget-tokens 200 --json")?;
        assert!(matches!(&request.verb, Verb::Why { text, .. } if text == "batch ledger writes"));
        assert_eq!((request.budget, request.json), (200, true));
        assert!(req("context a b").is_err());
        assert!(req("context a --offline").is_err());
        assert!(req("brief x --all").is_err());
        assert!(req("brief --budget-tokens 0").is_err());
        assert!(req("stale --at abc").is_err());
        assert!(req("impact x --direction sideways").is_err());
        assert!(req("impact x --include everything").is_err());
        assert!(req("status x --kind ticket").is_err());
        let listed = req("find --kind decision,ticket --project harbor");
        assert!(listed.is_ok_and(|r| matches!(
            r.verb,
            Verb::Find { signatures: false, ref text, ref filter }
                if text.is_empty() && filter.kinds == ["decision", "ticket"]
        )));
        assert!(req("find").is_err());
        assert!(req("find --project harbor").is_err());
        assert!(req("match --kind signature").is_err());
        assert!(req("find --kind signature --signatures").is_err());
        let found = req("match fsync took --kind signature");
        assert!(found.is_ok_and(|r| matches!(
            r.verb,
            Verb::Find { signatures: true, ref text, .. } if text == "fsync took"
        )));
        let impact = req("impact harbor-api --depth 3 --direction in --include inferred");
        assert!(impact.is_ok_and(|r| matches!(
            r.verb,
            Verb::Impact {
                depth: 3,
                inferred: true,
                ..
            }
        )));
        assert!(req("index verify").is_ok_and(|r| matches!(r.verb, Verb::IndexVerify)));
        assert_eq!(req("brief --help").err().as_deref(), Some("help"));
        Ok(())
    }
}
