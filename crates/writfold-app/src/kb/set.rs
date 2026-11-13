//! `status <id> --set STATE …`: parsing only; `writfold-kb` decides which
//! flags a state change takes.
use super::args::Common;
use super::writes::{get, need};
use std::path::PathBuf;
use writfold_kb::write::Write;
use writfold_kb::write::status::SetStatus;

/// The flags `status --set` accepts.
pub const FLAGS: &[&str] = &[
    "set",
    "by",
    "superseded-by",
    "note",
    "commit",
    "evidence",
    "run",
    "repo",
];

pub fn status(common: &Common, target: String) -> Result<Write, String> {
    Ok(Write::SetStatus(SetStatus {
        target,
        state: need(common, "status", "set")?,
        by: get(common, "by"),
        superseded_by: get(common, "superseded-by"),
        note: get(common, "note"),
        commit: get(common, "commit"),
        evidence: get(common, "evidence"),
        run: get(common, "run"),
        repo: get(common, "repo").map(PathBuf::from),
    }))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn decision_flags() -> Result<(), String> {
        let applied = verb("status d --set applied --commit demo@abc123 --by me")?;
        assert!(matches!(applied, Verb::Write(Write::SetStatus(s))
            if s.commit.as_deref() == Some("demo@abc123")));
        let verified = verb("status d --set verified --evidence gate:unit --run r1 --repo .")?;
        assert!(matches!(verified, Verb::Write(Write::SetStatus(s))
            if s.evidence.as_deref() == Some("gate:unit")
                && s.run.as_deref() == Some("r1")
                && s.repo.is_some()));
        assert!(verb("status d --set applied --ttl 3d").is_err());
        Ok(())
    }
}
