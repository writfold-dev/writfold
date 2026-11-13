//! `observe --source KIND --locator TEXT --status STATUS …`: parsing only;
//! `writfold-kb` validates and records.
use super::args::Common;
use super::writes::{get, need};
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

/// Value flags only `observe` takes (`--status`, `--note` and `--run` are
/// shared with other write verbs).
pub const VALUES: [&str; 5] = ["source", "locator", "revision", "for", "cite"];

/// The write flags `observe` accepts.
pub const FLAGS: &[&str] = &[
    "source", "locator", "status", "revision", "for", "note", "cite", "run",
];

pub fn verb(common: &Common, positional: &[String]) -> Result<Verb, String> {
    if !positional.is_empty() {
        return Err("observe takes flags only (--source, --locator, --status, …)".into());
    }
    Ok(Verb::Write(Write::Observe(Observe {
        source: need(common, "observe", "source")?,
        locator: need(common, "observe", "locator")?,
        status: need(common, "observe", "status")?,
        revision: get(common, "revision"),
        item: get(common, "for"),
        note: get(common, "note"),
        cite: get(common, "cite"),
        run: get(common, "run"),
    })))
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &[&str]) -> Result<Verb, String> {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        Ok(parse(&args, PathBuf::from("/"))?.ok_or("a request")?.verb)
    }

    #[test]
    fn observe_flags() -> Result<(), String> {
        let got = verb(&[
            "observe",
            "--source",
            "dash",
            "--locator",
            "p99 latency",
            "--status",
            "timeout",
            "--for",
            "review.perf",
            "--run",
            "r",
        ])?;
        assert!(matches!(
            got,
            Verb::Write(Write::Observe(ref o))
                if o.source == "dash" && o.locator == "p99 latency" && o.status == "timeout"
                    && o.item.as_deref() == Some("review.perf") && o.run.as_deref() == Some("r")
                    && o.cite.is_none()
        ));
        assert!(verb(&["observe", "--source", "dash", "--status", "ok"]).is_err());
        assert!(
            verb(&[
                "observe",
                "x",
                "--source",
                "d",
                "--locator",
                "l",
                "--status",
                "ok"
            ])
            .is_err()
        );
        assert!(verb(&["record", "ticket", "a-1", "--title", "t", "--source", "d"]).is_err());
        assert!(
            verb(&[
                "observe",
                "--source",
                "d",
                "--locator",
                "l",
                "--status",
                "ok",
                "--by",
                "me"
            ])
            .is_err()
        );
        Ok(())
    }
}
