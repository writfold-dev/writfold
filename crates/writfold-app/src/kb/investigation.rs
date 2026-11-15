//! `record investigation` and `propose investigation`: their flags and
//! parsing only; `writfold-kb` decides.
use super::args::Common;
use super::writes::{get, items, need};
use writfold_kb::write::Write;
use writfold_kb::write::investigation::Investigation;

/// The value flags only `record investigation` takes.
pub const VALUES: [&str; 5] = ["trigger", "tickets", "alerts", "signatures", "question"];

/// Every flag `record` accepts, for each of its kinds.
pub const RECORD: &[&str] = &[
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
    "trigger",
    "tickets",
    "alerts",
    "signatures",
    "question",
];

/// `record`'s flags plus `propose`'s own.
pub const PROPOSE: &[&str] = &[
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
    "trigger",
    "tickets",
    "alerts",
    "signatures",
    "question",
    "session",
    "run",
    "tracked",
    "local",
];

/// `record investigation --title T [--trigger …] …`.
pub fn write(common: &Common) -> Result<Write, String> {
    Ok(Write::Investigation(Investigation {
        title: need(common, "record investigation", "title")?,
        trigger: get(common, "trigger"),
        tickets: items(common, "tickets"),
        assets: items(common, "assets"),
        alerts: items(common, "alerts"),
        signatures: items(common, "signatures"),
        summary: get(common, "summary"),
        question: get(common, "question"),
    }))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &[&str]) -> Result<Verb, String> {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn record_and_propose_investigation() -> Result<(), String> {
        let args = [
            "record",
            "investigation",
            "--title",
            "Stalls",
            "--tickets",
            "HBR-1, HBR-2",
        ];
        let got = verb(&args)?;
        assert!(
            matches!(&got, Verb::Write(Write::Investigation(i)) if i.tickets == ["HBR-1", "HBR-2"])
        );
        let question = [
            "record",
            "investigation",
            "--title",
            "x",
            "--question",
            "why?",
        ];
        assert!(
            matches!(verb(&question)?, Verb::Write(Write::Investigation(i)) if i.question.as_deref() == Some("why?"))
        );
        assert!(verb(&["record", "investigation", "x", "--title", "t"]).is_err());
        assert!(verb(&["record", "investigation"]).is_err());
        let propose = [
            "propose",
            "investigation",
            "--title",
            "t",
            "--signatures",
            "a",
        ];
        assert!(matches!(verb(&propose)?, Verb::Proposal(_)));
        assert!(verb(&["status", "x", "--trigger", "y"]).is_err());
        Ok(())
    }
}
