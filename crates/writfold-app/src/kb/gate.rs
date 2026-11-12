//! `writfold kb gate run|status|verify|lines`: arguments only; the gate
//! logic is in `writfold_kb::gate`.
use super::args::Common;
use super::writes::get;
use std::path::PathBuf;
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

pub const USAGE: &str = "usage: gate run <id> [--run ID] [--repo DIR] | gate status [--run ID] [--repo DIR] | gate verify <receipt> [--repo DIR] | gate lines [--run ID] [--repo DIR]";

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
        for bad in [
            "gate",
            "gate run",
            "gate run a b",
            "gate status x",
            "gate verify x --run r",
            "gate status --note x",
            "gate sing",
        ] {
            assert!(verb(bad).is_err(), "{bad}");
        }
        Ok(())
    }
}
