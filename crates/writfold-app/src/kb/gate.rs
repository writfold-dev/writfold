//! `writfold kb gate run|status|verify|lines|attest|waive|cover`:
//! arguments only; the gate logic is in `writfold_kb::gate`.
use super::args::Common;
use super::writes::get;
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::gate::checklist::{Cover, Outcome};
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

pub const USAGE: &str = "usage: gate run <id> [--run ID] [--repo DIR] | gate attest <id> --statement TEXT --by WHO | gate waive <id> --reason TEXT --by WHO | gate cover <id> <item> supported REF… [--note TEXT] | gate cover <id> <item> unsupported|not-applicable --reason TEXT (each with --by WHO) | gate status [--run ID] [--repo DIR] | gate verify <receipt> [--repo DIR] | gate lines [--run ID] [--repo DIR]";

/// The gate verb, or why the arguments do not name one.
pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    let run = get(common, "run");
    let repo = get(common, "repo").map(PathBuf::from);
    let verb = match positional {
        [sub, id] if sub == "run" => GateVerb::Run {
            id: id.clone(),
            run,
            repo,
        },
        [sub, _, _, _, ..] if sub == "cover" && get(common, "statement").is_some() => {
            return Err("gate cover takes --reason or references, not --statement".into());
        }
        [sub, id, item, outcome, references @ ..] if sub == "cover" => GateVerb::Cover(Cover {
            gate: id.clone(),
            item: item.clone(),
            outcome: Outcome::parse(outcome)
                .ok_or("gate cover <id> <item> takes supported, unsupported or not-applicable")?,
            references: references.to_vec(),
            reason: get(common, "reason"),
            note: get(common, "note"),
            by: get(common, "by").ok_or("gate cover needs --by WHO")?,
            run,
            repo,
        }),
        _ if get(common, "note").is_some() => {
            return Err("--note belongs to gate cover".into());
        }
        [sub, id] if sub == "attest" || sub == "waive" => {
            let (kind, flag, other) = match sub.as_str() {
                "attest" => (Kind::Attested, "statement", "reason"),
                _ => (Kind::Waived, "reason", "statement"),
            };
            if get(common, other).is_some() {
                return Err(format!("gate {sub} takes --{flag}, not --{other}"));
            }
            GateVerb::Say(Say {
                kind,
                id: id.clone(),
                text: get(common, flag).ok_or(format!("gate {sub} needs --{flag}"))?,
                by: get(common, "by").ok_or(format!("gate {sub} needs --by WHO"))?,
                run,
                repo,
            })
        }
        _ if get(common, "statement").is_some()
            || get(common, "reason").is_some()
            || get(common, "by").is_some() =>
        {
            return Err("--statement, --reason and --by belong to gate attest|waive|cover".into());
        }
        [sub] if sub == "status" => GateVerb::Status { run, repo },
        [sub] if sub == "lines" => GateVerb::Lines { run, repo },
        [sub, receipt] if sub == "verify" => {
            if run.is_some() {
                return Err("gate verify takes the receipt, not --run".into());
            }
            GateVerb::Verify {
                receipt: receipt.clone(),
                repo,
            }
        }
        _ => return Err(USAGE.into()),
    };
    Ok(Verb::Gate(verb))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::gate::GateVerb;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn gate_verbs() -> Result<(), String> {
        let run = verb("gate run unit --repo . --run r1")?;
        assert!(matches!(
            run,
            Verb::Gate(GateVerb::Run { ref id, run: Some(ref r), repo: Some(_) }) if id == "unit" && r == "r1"
        ));
        assert!(matches!(
            verb("gate status")?,
            Verb::Gate(GateVerb::Status {
                run: None,
                repo: None
            })
        ));
        assert!(matches!(
            verb("gate lines --run r")?,
            Verb::Gate(GateVerb::Lines { .. })
        ));
        assert!(matches!(
            verb("gate verify runs/r/gates/unit-x.json")?,
            Verb::Gate(GateVerb::Verify { .. })
        ));
        assert!(matches!(
            verb("gate waive unit --reason flaky --by me")?,
            Verb::Gate(GateVerb::Say(ref s)) if s.id == "unit" && s.text == "flaky"
        ));
        assert!(matches!(
            verb("gate attest review --statement read --by me --run r")?,
            Verb::Gate(GateVerb::Say(_))
        ));
        assert!(matches!(
            verb("gate cover review errors supported gate:unit runs/r/gates/unit-x.json --by me --note n")?,
            Verb::Gate(GateVerb::Cover(ref c)) if c.references.len() == 2 && c.note.as_deref() == Some("n")
        ));
        assert!(matches!(
            verb("gate cover review perf not-applicable --reason none --by me")?,
            Verb::Gate(GateVerb::Cover(ref c)) if c.references.is_empty() && c.reason.as_deref() == Some("none")
        ));
        for bad in [
            "gate attest review --reason x --by me",
            "gate waive unit --reason x",
            "gate status --by me",
            "gate",
            "gate run",
            "gate run a b",
            "gate status x",
            "gate verify x --run r",
            "gate status --note x",
            "gate waive unit --reason x --by me --note y",
            "gate cover review errors supported gate:unit",
            "gate cover review errors met gate:unit --by me",
            "gate cover review errors",
            "gate sing",
        ] {
            assert!(verb(bad).is_err(), "{bad}");
        }
        Ok(())
    }
}
