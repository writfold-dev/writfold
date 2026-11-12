//! `writfold kb deliver prepare|check|attest|waive|status`: arguments only;
//! the logic is in `writfold_kb::deliver`. Nothing is sent.
use super::args::Common;
use super::writes::get;
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::gate::statement::Kind;

pub const USAGE: &str = "usage: deliver prepare --mode pr-comment|slack-summary --from FILE [--audience internal|external] [--destination NAME] [--run ID] | deliver check [CANDIDATE] [--check NAME] [--run ID] | deliver attest [CANDIDATE] --check NAME --statement TEXT --by WHO [--run ID] | deliver waive [CANDIDATE] --check NAME --reason TEXT --by WHO [--run ID] | deliver status [CANDIDATE] [--run ID]";

/// The flags each deliver verb takes, beyond `--run`.
fn flags(sub: &str) -> &'static [&'static str] {
    match sub {
        "prepare" => &["mode", "from", "audience", "destination"],
        "check" => &["check"],
        "attest" => &["check", "statement", "by"],
        "waive" => &["check", "reason", "by"],
        _ => &[],
    }
}

/// The deliver verb, or why the arguments do not name one.
pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    let Some((sub, rest)) = positional.split_first() else {
        return Err(USAGE.into());
    };
    let allowed = flags(sub);
    if let Some(flag) = common
        .values
        .keys()
        .find(|k| *k != "run" && !allowed.contains(&k.as_str()))
    {
        return Err(format!("--{flag} does not apply to deliver {sub}"));
    }
    let run = get(common, "run");
    let need = |key: &str| get(common, key).ok_or(format!("deliver {sub} needs --{key}"));
    let candidate = match rest {
        [] => None,
        [one] => Some(one.clone()),
        _ => return Err(format!("deliver {sub} takes at most one candidate")),
    };
    let verb = match sub.as_str() {
        "prepare" if candidate.is_none() => DeliverVerb::Prepare(Prepare {
            mode: need("mode")?,
            from: PathBuf::from(need("from")?),
            audience: get(common, "audience"),
            destination: get(common, "destination"),
            run,
        }),
        "check" => DeliverVerb::Check {
            candidate,
            check: get(common, "check"),
            run,
        },
        "attest" | "waive" => {
            let (kind, flag) = match sub.as_str() {
                "attest" => (Kind::Attested, "statement"),
                _ => (Kind::Waived, "reason"),
            };
            DeliverVerb::Say {
                kind,
                candidate,
                check: need("check")?,
                text: need(flag)?,
                by: need("by")?,
                run,
            }
        }
        "status" => DeliverVerb::Status { candidate, run },
        _ => return Err(USAGE.into()),
    };
    Ok(Verb::Deliver(verb))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::deliver::DeliverVerb;

    fn verb(args: &[&str]) -> Result<Verb, String> {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn deliver_verbs() -> Result<(), String> {
        let prepare = verb(&[
            "deliver",
            "prepare",
            "--mode",
            "pr-comment",
            "--from",
            "c.md",
            "--audience",
            "external",
            "--destination",
            "harbor#42",
            "--run",
            "r",
        ])?;
        assert!(matches!(
            prepare,
            Verb::Deliver(DeliverVerb::Prepare(ref p))
                if p.mode == "pr-comment" && p.audience.as_deref() == Some("external")
        ));
        assert!(matches!(
            verb(&[
                "deliver",
                "check",
                "abc123",
                "--check",
                "paragraphs.bounded"
            ])?,
            Verb::Deliver(DeliverVerb::Check {
                candidate: Some(_),
                check: Some(_),
                ..
            })
        ));
        assert!(matches!(
            verb(&[
                "deliver", "waive", "--check", "x", "--reason", "why", "--by", "me"
            ])?,
            Verb::Deliver(DeliverVerb::Say { .. })
        ));
        assert!(matches!(
            verb(&["deliver", "status"])?,
            Verb::Deliver(DeliverVerb::Status {
                candidate: None,
                run: None
            })
        ));
        for bad in [
            &["deliver"][..],
            &["deliver", "send"],
            &["deliver", "prepare", "--mode", "pr-comment"],
            &["deliver", "prepare", "x", "--mode", "m", "--from", "f"],
            &["deliver", "status", "--mode", "m"],
            &[
                "deliver", "attest", "--check", "x", "--reason", "r", "--by", "me",
            ],
            &["deliver", "waive", "--check", "x", "--by", "me"],
            &["deliver", "status", "a", "b"],
        ] {
            assert!(verb(bad).is_err(), "{bad:?}");
        }
        Ok(())
    }
}
