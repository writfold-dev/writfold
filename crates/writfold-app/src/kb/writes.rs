//! The write verbs' arguments: `checkpoint`, `record ticket|decision|link|investigation`,
//! `status --set` and `cite make`. Parsing only; `writfold-kb` decides.
use super::args::Common;
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::record::{Decision, Link, Ticket};

/// Flags that take a value.
pub const VALUES: [&str; 42] = [
    "artifact",
    "idle",
    "example",
    "mark",
    "since-mark",
    "rule",
    "since",
    "from",
    "session",
    "audience",
    "destination",
    "check",
    "statement",
    "reason",
    "note",
    "worktree",
    "repo",
    "from-draft",
    "run",
    "task",
    "mode",
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
    "set",
    "by",
    "superseded-by",
    "sym",
    "ttl",
    "commit",
    "ref",
    "to",
];
/// Flags without a value.
pub const SWITCHES: [&str; 12] = [
    "each",
    "here",
    "deny-warnings",
    "records",
    "eval",
    "open",
    "new-run",
    "allow-dirty",
    "dry-run",
    "abandon",
    "plan",
    "gates",
];

/// The write flags each verb accepts.
pub fn allowed(verb: &str) -> &'static [&'static str] {
    match verb {
        "runs" => &["open", "plan", "idle"],
        "find" => &["eval", "each"],
        "resume" => &["here"],
        "guard" => &["rule", "since"],
        "delta" => &["mark", "since-mark", "since"],
        "checkpoint" => &[
            "note",
            "from-draft",
            "run",
            "task",
            "mode",
            "new-run",
            "artifact",
        ],
        "record" => super::investigation::RECORD,
        "status" => super::set::FLAGS,
        "cite" => &["allow-dirty", "sym", "ttl"],
        "gate" => &["run", "repo", "statement", "reason", "by", "note", "from"],
        "deliver" => &[
            "run",
            "mode",
            "from",
            "audience",
            "destination",
            "check",
            "statement",
            "reason",
            "by",
            "ref",
            "to",
        ],
        "plan" => &["run", "repo", "from", "reason", "by", "again", "gates"],
        "observe" => super::observe::FLAGS,
        _ => super::proposals::allowed(verb),
    }
}

pub fn get(common: &Common, key: &str) -> Option<String> {
    common.values.get(key).cloned()
}

pub fn need(common: &Common, verb: &str, key: &str) -> Result<String, String> {
    get(common, key).ok_or(format!("{verb} needs --{key}"))
}

pub fn items(common: &Common, key: &str) -> Vec<String> {
    get(common, key)
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn switch(common: &Common, name: &str) -> bool {
    common.switches.iter().any(|s| s == name)
}

/// `runs`, `resume`, the write verbs, then `propose`, `proposals` and
/// `check`; `None` for every other verb.
pub fn runs_and_writes(
    name: &str,
    common: &Common,
    positional: &[String],
) -> Option<Result<Verb, String>> {
    Some(match (name, positional) {
        ("runs", []) if common.values.contains_key("idle") => super::lists::idle(common),
        ("runs", []) => Ok(Verb::Runs {
            open: switch(common, "open"),
            project: common.project.clone(),
            plan: switch(common, "plan"),
        }),
        ("resume", _) => super::lists::resume(switch(common, "here"), positional),
        ("runs", _) => Err(format!("{name}: too many arguments")),
        ("status", [target]) if common.values.contains_key("set") => {
            super::set::status(common, target.clone()).map(Verb::Write)
        }
        ("checkpoint" | "record", _) => verb(name, common, positional).map(Verb::Write),
        ("cite" | "checker", _) => super::cite::verb(name, common, positional),
        ("deliver", _) => super::deliver::verb(common, positional),
        ("plan", _) => super::plan::verb(common, positional),
        ("guard", _) => super::guard::verb(common, positional),
        ("delta", _) => super::lists::delta(common, positional),
        ("observe", _) => super::observe::verb(common, positional),
        _ => return super::proposals::verb(name, common, positional),
    })
}

pub fn verb(name: &str, common: &Common, positional: &[String]) -> Result<Write, String> {
    match (name, positional) {
        ("checkpoint", []) => super::checkpoint::checkpoint(common),
        ("checkpoint", _) => Err("checkpoint takes flags only (--run, --new-run, --note)".into()),
        ("record", [kind, rest @ ..]) => record(kind, common, rest),
        _ => Err("usage: record ticket|decision|link|investigation …".into()),
    }
}

pub fn record(kind: &str, common: &Common, rest: &[String]) -> Result<Write, String> {
    Ok(match (kind, rest) {
        ("ticket", [ticket]) => Write::Ticket(Ticket {
            ticket: ticket.clone(),
            title: need(common, "record ticket", "title")?,
            assets: items(common, "assets"),
            status: get(common, "status"),
            summary: get(common, "summary"),
            prs: items(common, "prs"),
        }),
        ("decision", []) => Write::Decision(Decision {
            title: need(common, "record decision", "title")?,
            target_source: need(common, "record decision", "target-source")?,
            project: common.project.clone(),
            references: items(common, "references"),
            context: get(common, "context"),
            evidence: get(common, "evidence"),
            change: get(common, "change"),
            why: get(common, "why"),
            origin: get(common, "origin"),
        }),
        ("investigation", []) => super::investigation::write(common)?,
        ("link", [rel, from, to]) => Write::Link(Link {
            rel: rel.clone(),
            from: from.clone(),
            to: to.clone(),
            status: get(common, "status"),
            evidence: items(common, "evidence"),
            applicability: get(common, "applicability"),
        }),
        _ => {
            return Err(
                "usage: record ticket <ID> | record decision | record link <type> <from> <to> | record investigation"
                    .into(),
            );
        }
    })
}

#[cfg(test)]
mod tests {
    use writfold_kb::Verb;
    use writfold_kb::write::Write;

    fn verb(args: &str) -> Result<Verb, String> {
        let args: Vec<String> = args.split(' ').map(str::to_owned).collect();
        super::super::args::verb_of(&args)
    }

    #[test]
    fn write_verbs() -> Result<(), String> {
        let new = verb("checkpoint --new-run --project p --task t --note n")?;
        assert!(matches!(new, Verb::Write(Write::Checkpoint(c)) if c.new_run.is_some()));
        assert!(verb("checkpoint --project p --note n").is_err());
        assert!(verb("checkpoint --run r --new-run --project p --task t --note n").is_err());
        let ticket = verb("record ticket ab-1 --title x --assets a,b")?;
        assert!(matches!(ticket, Verb::Write(Write::Ticket(t)) if t.assets == ["a", "b"]));
        assert!(verb("record ticket ab-1 --set done").is_err());
        let set = verb("status AB-1 --set done --by me")?;
        assert!(matches!(set, Verb::Write(Write::SetStatus(s)) if s.state == "done"));
        assert!(matches!(verb("status AB-1")?, Verb::Status { .. }));
        let cite = verb("cite make demo src/a.rs 3-5 --allow-dirty")?;
        assert!(matches!(cite, Verb::Write(Write::CiteMake(c)) if c.first == 3 && c.allow_dirty));
        assert!(verb("cite make demo src/a.rs x").is_err());
        assert!(matches!(
            verb("runs --open")?,
            Verb::Runs { open: true, .. }
        ));
        assert!(matches!(verb("resume")?, Verb::Resume { run: None }));
        Ok(())
    }

    #[test]
    fn run_positions_compose_with_filters_not_other_verbs() -> Result<(), String> {
        assert!(matches!(
            verb("runs --plan --open --project harbor")?,
            Verb::Runs { open: true, plan: true, project: Some(p) } if p == "harbor"
        ));
        assert!(verb("resume --plan").is_err());
        assert!(verb("runs --gates").is_err());
        Ok(())
    }
}
