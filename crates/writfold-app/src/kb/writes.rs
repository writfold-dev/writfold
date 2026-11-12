//! The write verbs' arguments: `checkpoint`, `record ticket|decision|link`,
//! `status --set` and `cite make`. Parsing only; `writfold-kb` decides.
use super::args::Common;
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::cite::CiteMake;
use writfold_kb::write::record::{Decision, Link, Ticket};
use writfold_kb::write::status::SetStatus;

/// Flags that take a value.
pub const VALUES: [&str; 32] = [
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
];
/// Flags without a value.
pub const SWITCHES: [&str; 5] = ["open", "new-run", "allow-dirty", "dry-run", "abandon"];

/// The write flags each verb accepts.
pub fn allowed(verb: &str) -> &'static [&'static str] {
    match verb {
        "runs" => &["open"],
        "checkpoint" => &["note", "from-draft", "run", "task", "mode", "new-run"],
        "record" => &[
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
        ],
        "status" => &["set", "by", "superseded-by", "note"],
        "cite" => &["allow-dirty", "sym", "ttl"],
        "gate" => &["run", "repo", "statement", "reason", "by"],
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
        ],
        "edit" => &["run", "worktree", "dry-run", "abandon"],
        "plan" => &["run", "repo", "from", "reason", "by", "again"],
        _ => super::proposals::allowed(verb),
    }
}

pub fn get(common: &Common, key: &str) -> Option<String> {
    common.values.get(key).cloned()
}

pub fn need(common: &Common, verb: &str, key: &str) -> Result<String, String> {
    get(common, key).ok_or(format!("{verb} needs --{key}"))
}

fn items(common: &Common, key: &str) -> Vec<String> {
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
        ("runs", []) => Ok(Verb::Runs {
            open: switch(common, "open"),
            project: common.project.clone(),
        }),
        ("resume", []) => Ok(Verb::Resume { run: None }),
        ("resume", [run]) => Ok(Verb::Resume {
            run: Some(run.clone()),
        }),
        ("runs" | "resume", _) => Err(format!("{name}: too many arguments")),
        ("status", [target]) if common.values.contains_key("set") => {
            status(common, target.clone()).map(Verb::Write)
        }
        ("checkpoint" | "record" | "cite", _) => verb(name, common, positional).map(Verb::Write),
        ("edit", _) => super::edit::verb(common, positional),
        ("deliver", _) => super::deliver::verb(common, positional),
        ("plan", _) => super::plan::verb(common, positional),
        _ => return super::proposals::verb(name, common, positional),
    })
}

/// `status <id> --set STATE …`.
pub fn status(common: &Common, target: String) -> Result<Write, String> {
    Ok(Write::SetStatus(SetStatus {
        target,
        state: need(common, "status", "set")?,
        by: get(common, "by"),
        superseded_by: get(common, "superseded-by"),
        note: get(common, "note"),
    }))
}

pub fn verb(name: &str, common: &Common, positional: &[String]) -> Result<Write, String> {
    match (name, positional) {
        ("checkpoint", []) => super::checkpoint::checkpoint(common),
        ("checkpoint", _) => Err("checkpoint takes flags only (--run, --new-run, --note)".into()),
        ("record", [kind, rest @ ..]) => record(kind, common, rest),
        ("cite", [make, repo, path, range]) if make == "make" => cite(common, repo, path, range),
        ("cite", _) => Err("usage: cite make <repo> <path> <a-b>".into()),
        _ => Err("usage: record ticket|decision|link …".into()),
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
                "usage: record ticket <ID> | record decision | record link <type> <from> <to>"
                    .into(),
            );
        }
    })
}

fn cite(common: &Common, repo: &str, path: &str, range: &str) -> Result<Write, String> {
    let (a, b) = range.split_once('-').unwrap_or((range, range));
    let (Ok(first), Ok(last)) = (a.parse::<u32>(), b.parse::<u32>()) else {
        return Err(format!("{range:?} is not a line range a-b"));
    };
    Ok(Write::CiteMake(CiteMake {
        repo: repo.to_owned(),
        path: path.to_owned(),
        first,
        last,
        allow_dirty: switch(common, "allow-dirty"),
        sym: get(common, "sym"),
        ttl: get(common, "ttl"),
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
}
