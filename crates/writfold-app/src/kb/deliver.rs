//! `writfold kb deliver modes|prepare|check|attest|waive|withdraw|payload|posted|match|status`: arguments only;
//! the logic is in `writfold_kb::deliver`. Nothing is sent.
use super::args::Common;
use super::writes::get;
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::gate::statement::Kind;

pub const USAGE: &str = "usage: deliver modes [MODE] | deliver prepare --mode MODE --from FILE|- [--audience internal|external] [--destination NAME] [--invocation ID] [--run ID] | deliver check [CANDIDATE] [--mode MODE] [--check NAME] [--run ID] | deliver attest [CANDIDATE] [--mode MODE] --check NAME --statement TEXT --by WHO [--run ID] | deliver waive [CANDIDATE] [--mode MODE] --check NAME --reason TEXT --by WHO [--run ID] | deliver withdraw [CANDIDATE] [--mode MODE] --reason TEXT --by WHO [--run ID] | deliver payload [CANDIDATE] [--mode MODE] [--to FILE] [--run ID] | deliver posted [CANDIDATE] [--mode MODE] --ref TEXT --by WHO [--run ID] | deliver match --from FILE|- [--mode MODE] [--run ID] | deliver status [CANDIDATE] [--all] [--run ID]";

/// The flags each deliver verb takes, beyond `--run`.
fn flags(sub: &str) -> &'static [&'static str] {
    match sub {
        "prepare" => &["mode", "from", "audience", "destination", "invocation"],
        "check" => &["check", "mode"],
        "attest" => &["check", "statement", "by", "mode"],
        "waive" => &["check", "reason", "by", "mode"],
        "withdraw" => &["reason", "by", "mode"],
        "posted" => &["ref", "by", "mode"],
        "payload" => &["to", "mode"],
        "match" => &["from", "mode"],
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
    if common.all && sub != "status" {
        return Err(format!("--all does not apply to deliver {sub}"));
    }
    if sub == "modes" {
        return modes(common, rest);
    }
    let run = get(common, "run");
    let mode = get(common, "mode");
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
            invocation: get(common, "invocation"),
        }),
        "check" => DeliverVerb::Check {
            candidate,
            check: get(common, "check"),
            run,
            mode,
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
                mode,
            }
        }
        "status" => DeliverVerb::Status {
            candidate,
            run,
            all: common.all,
        },
        other => records(common, other, candidate, (run, mode))?,
    };
    Ok(Verb::Deliver(verb))
}

/// `deliver modes [MODE]`.
fn modes(common: &Common, rest: &[String]) -> Result<Verb, String> {
    match (rest, common.values.keys().next()) {
        (_, Some(flag)) => Err(format!("--{flag} does not apply to deliver modes")),
        ([], None) => Ok(Verb::Deliver(DeliverVerb::Modes { mode: None })),
        ([one], None) => Ok(Verb::Deliver(DeliverVerb::Modes {
            mode: Some(one.clone()),
        })),
        _ => Err("deliver modes takes at most one mode".into()),
    }
}

/// `deliver withdraw`, `posted`, `payload` and `match`.
fn records(
    common: &Common,
    sub: &str,
    candidate: Option<String>,
    (run, mode): (Option<String>, Option<String>),
) -> Result<DeliverVerb, String> {
    let need = |key: &str| get(common, key).ok_or(format!("deliver {sub} needs --{key}"));
    Ok(match sub {
        "withdraw" => DeliverVerb::Withdraw {
            candidate,
            reason: need("reason")?,
            by: need("by")?,
            run,
            mode,
        },
        "posted" => DeliverVerb::Posted {
            candidate,
            reference: need("ref")?,
            by: need("by")?,
            run,
            mode,
        },
        "payload" => DeliverVerb::Payload {
            candidate,
            to: get(common, "to").map(PathBuf::from),
            run,
            mode,
        },
        "match" if candidate.is_none() => DeliverVerb::Match {
            from: PathBuf::from(need("from")?),
            mode,
            run,
        },
        _ => return Err(USAGE.into()),
    })
}

#[cfg(test)]
mod tests {
    use writfold_kb::Verb;
    use writfold_kb::deliver::DeliverVerb;

    fn verb(args: &[&str]) -> Result<Verb, String> {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        super::super::args::verb_of(&args)
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
                run: None,
                all: false
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
