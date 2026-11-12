//! One `writfold kb` verb from its name, flags and positionals.
use super::args::Common;
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;
use writfold_kb::verbs::impact::Direction;

fn why_sections() -> Vec<String> {
    std::env::var("WRITFOLD_KB_WHY_SECTIONS")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The typed verb, or why the arguments do not name one.
pub fn verb(name: &str, common: &Common, positional: &[String]) -> Result<Verb, String> {
    if let Some(verb) = super::writes::runs_and_writes(name, common, positional) {
        return verb;
    }
    if name == "gate" {
        return super::gate::verb(common, positional);
    }
    let one = |what: &str| -> Result<String, String> {
        match positional {
            [single] => Ok(single.clone()),
            _ => Err(format!("{name} takes exactly one {what}")),
        }
    };
    let optional = || -> Result<Option<String>, String> {
        match positional {
            [] => Ok(None),
            [single] => Ok(Some(single.clone())),
            _ => Err(format!("{name} takes at most one argument")),
        }
    };
    let text = |what: &str| -> Result<String, String> {
        if positional.is_empty() {
            Err(format!("{name} takes {what}"))
        } else {
            Ok(positional.join(" "))
        }
    };
    Ok(match name {
        "brief" => {
            let project = optional()?;
            if project.is_some() && common.all {
                return Err("brief takes a project or --all, not both".into());
            }
            Verb::Brief {
                project,
                all: common.all,
            }
        }
        "context" => Verb::Context {
            target: one("target")?,
            depth: common.depth.unwrap_or(2),
        },
        "why" => Verb::Why {
            text: text("an ID or some text")?,
            sections: why_sections(),
        },
        "stale" => Verb::Stale {
            scope: optional()?,
            offline: common.offline,
        },
        "find" | "match" => Verb::Find {
            text: text("some text")?,
            filter: Filter {
                kinds: common.kinds.clone(),
                project: common.project.clone(),
            },
            signatures: common.signatures || name == "match",
        },
        "status" => Verb::Status {
            target: one("ID or project")?,
        },
        "impact" => impact(common, one("target")?)?,
        "uses" => Verb::Uses {
            target: one("target")?,
        },
        "timeline" => Verb::Timeline {
            target: one("target")?,
        },
        "index" => match one("subcommand (status, rebuild or verify)")?.as_str() {
            "status" => Verb::IndexStatus,
            "rebuild" => Verb::IndexRebuild,
            "verify" => Verb::IndexVerify,
            other => return Err(format!("unknown index subcommand {other}")),
        },
        other => return Err(format!("unknown verb {other}")),
    })
}

fn impact(common: &Common, target: String) -> Result<Verb, String> {
    let direction = match common.direction.as_deref() {
        None => Direction::Both,
        Some(text) => Direction::parse(text)
            .ok_or_else(|| format!("--direction must be in, out or both, not {text}"))?,
    };
    let inferred = match common.include.as_deref() {
        None => false,
        Some("inferred") => true,
        Some(other) => return Err(format!("--include takes only inferred, not {other}")),
    };
    Ok(Verb::Impact {
        target,
        depth: common.depth.unwrap_or(2),
        direction,
        inferred,
    })
}
