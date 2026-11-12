//! `propose …`, `proposals list|show|accept|drop` and `check`: parsing
//! only; the queue and the composed check live in `writfold-kb`.
use super::args::Common;
use super::writes::{get, record};
use writfold_kb::Verb;
use writfold_kb::proposal::{ProposalVerb, Propose};

/// The environment variable naming the session when `--session` is not
/// given.
pub const SESSION_ENV: &str = "WRITFOLD_SESSION";

/// The write flags each verb here accepts.
pub fn allowed(verb: &str) -> &'static [&'static str] {
    match verb {
        "propose" => &[
            "title",
            "assets",
            "status",
            "summary",
            "prs",
            "origin",
            "context",
            "evidence",
            "change",
            "why",
            "target-source",
            "references",
            "applicability",
            "session",
            "run",
        ],
        _ => &[],
    }
}

/// The typed verb for `propose`, `proposals` and `check`; `None` for every
/// other verb.
pub fn verb(name: &str, common: &Common, positional: &[String]) -> Option<Result<Verb, String>> {
    let verb = match (name, positional) {
        ("propose", [kind, rest @ ..]) => record(kind, common, rest).map(|write| {
            ProposalVerb::Propose(Box::new(Propose {
                write,
                session: get(common, "session")
                    .or_else(|| std::env::var(SESSION_ENV).ok())
                    .filter(|s| !s.is_empty()),
                run: get(common, "run"),
            }))
        }),
        ("propose", []) => Err("usage: propose ticket|decision|link … (as record)".into()),
        ("proposals", [] | [_]) => {
            let sub = positional.first().map_or("list", String::as_str);
            match sub {
                "list" => Ok(ProposalVerb::List),
                _ => Err("usage: proposals list | proposals show|accept|drop <id>".into()),
            }
        }
        ("proposals", [sub, id]) => match sub.as_str() {
            "show" => Ok(ProposalVerb::Show { id: id.clone() }),
            "accept" => Ok(ProposalVerb::Accept { id: id.clone() }),
            "drop" => Ok(ProposalVerb::Drop { id: id.clone() }),
            other => Err(format!("unknown proposals subcommand {other}")),
        },
        ("proposals", _) => Err("proposals: too many arguments".into()),
        ("check", []) => return Some(Ok(Verb::Check { scope: None })),
        ("check", [scope]) => {
            return Some(Ok(Verb::Check {
                scope: Some(scope.clone()),
            }));
        }
        ("check", _) => Err("check takes at most one project or path".into()),
        _ => return None,
    };
    Some(verb.map(Verb::Proposal))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::proposal::ProposalVerb;
    use writfold_kb::write::Write;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn proposal_verbs() -> Result<(), String> {
        let made = verb("propose ticket AB-1 --title t --assets a --session s1 --run r")?;
        assert!(matches!(&made, Verb::Proposal(ProposalVerb::Propose(p))
            if p.session.as_deref() == Some("s1") && p.run.as_deref() == Some("r")
                && matches!(&p.write, Write::Ticket(t) if t.assets == ["a"])));
        let decision = verb("propose decision --title t --target-source s --project p")?;
        assert!(matches!(&decision, Verb::Proposal(ProposalVerb::Propose(p))
            if matches!(&p.write, Write::Decision(d) if d.project.as_deref() == Some("p"))));
        assert!(matches!(
            verb("proposals")?,
            Verb::Proposal(ProposalVerb::List)
        ));
        assert!(matches!(
            verb("proposals accept 0123")?,
            Verb::Proposal(ProposalVerb::Accept { .. })
        ));
        assert!(verb("proposals accept").is_err());
        assert!(verb("proposals list --session s").is_err());
        assert!(verb("record ticket AB-1 --title t --session s").is_err());
        assert!(verb("propose status AB-1").is_err());
        assert!(matches!(verb("check")?, Verb::Check { scope: None }));
        assert!(matches!(
            verb("check harbor")?,
            Verb::Check { scope: Some(_) }
        ));
        assert!(verb("check a b").is_err());
        Ok(())
    }
}
