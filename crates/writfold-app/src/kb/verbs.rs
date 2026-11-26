//! One `writfold kb` verb from its name, flags and positionals.
use super::args::Common;
use std::path::Path;
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
pub fn verb(
    name: &str,
    common: &Common,
    positional: &[String],
    cwd: &Path,
) -> Result<Verb, String> {
    if let Some(verb) = super::writes::runs_and_writes(name, common, positional) {
        return verb;
    }
    if name == "gate" {
        return super::gate::verb(common, positional);
    }
    if name == "handoff" {
        return handoff(common, positional);
    }
    if name == "catalog" {
        return catalog(positional);
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
        "stale" if common.record => Verb::StaleRecord { scope: optional()? },
        "stale" => Verb::Stale {
            scope: optional()?,
            offline: common.offline,
            cached: common.cached,
        },
        "find" if common.switches.iter().any(|s| s == "eval") => eval(common, positional, cwd)?,
        "find" | "match" => find(name, common, positional)?,
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
        "repos" => repos(common, positional)?,
        "index" => match one("subcommand (status, rebuild or verify)")?.as_str() {
            "status" => Verb::IndexStatus,
            "rebuild" => Verb::IndexRebuild,
            "verify" => Verb::IndexVerify,
            other => return Err(format!("unknown index subcommand {other}")),
        },
        other => return Err(format!("unknown verb {other}")),
    })
}

/// `find TEXT` and `match TEXT`; `find --kind K` with no text lists records.
fn find(name: &str, common: &Common, positional: &[String]) -> Result<Verb, String> {
    if common.switches.iter().any(|s| s == "each") {
        if common.limit.is_some() {
            return Err(
                "find --each shows the first five rows of each query; --limit is for one query"
                    .into(),
            );
        }
        if positional.is_empty() {
            return Err("find --each takes one or more queries".into());
        }
        return Ok(Verb::FindEach {
            texts: positional.to_vec(),
            filter: Filter {
                kinds: common.kinds.clone(),
                project: common.project.clone(),
                limit: None,
            },
            signatures: common.signatures,
        });
    }
    let listing = name == "find" && !common.kinds.is_empty() && !common.signatures;
    if positional.is_empty() && !listing {
        return Err(format!("{name} takes some text"));
    }
    Ok(Verb::Find {
        text: positional.join(" "),
        filter: Filter {
            kinds: common.kinds.clone(),
            project: common.project.clone(),
            limit: common.limit,
        },
        signatures: common.signatures || name == "match",
    })
}

/// `find --eval [FILE]`: FILE from the working directory; no other flag.
fn eval(common: &Common, positional: &[String], cwd: &Path) -> Result<Verb, String> {
    if common.signatures
        || !common.kinds.is_empty()
        || common.project.is_some()
        || common.limit.is_some()
    {
        return Err("find --eval takes only a query file".into());
    }
    match positional {
        [] => Ok(Verb::FindEval { file: None }),
        [file] => Ok(Verb::FindEval {
            file: Some(cwd.join(file)),
        }),
        _ => Err("find --eval takes at most one file".into()),
    }
}

fn repos(common: &Common, positional: &[String]) -> Result<Verb, String> {
    match positional {
        [] => Ok(Verb::Repos {
            missing: common.missing,
        }),
        _ => Err("repos takes no arguments".into()),
    }
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
        manifest: super::writes::get(common, "manifest").map(std::path::PathBuf::from),
    })
}

/// `handoff check --policy REV --candidate REV [--run ID] [--repo DIR]`.
fn handoff(common: &Common, positional: &[String]) -> Result<Verb, String> {
    if positional != ["check"] {
        return Err(
            "usage: handoff check --policy REV --candidate REV [--run ID] [--repo DIR]".into(),
        );
    }
    Ok(Verb::Handoff {
        policy: super::writes::need(common, "handoff check", "policy")?,
        candidate: super::writes::need(common, "handoff check", "candidate")?,
        run: super::writes::get(common, "run"),
        repo: super::writes::get(common, "repo").map(std::path::PathBuf::from),
    })
}

/// `catalog diff NAME` and `catalog check [NAME]` (#86).
fn catalog(positional: &[String]) -> Result<Verb, String> {
    use writfold_kb::catalog::CatalogVerb;
    Ok(Verb::Catalog(match positional {
        [d, name] if d == "diff" => CatalogVerb::Diff { name: name.clone() },
        [c] if c == "check" => CatalogVerb::Check { name: None },
        [c, name] if c == "check" => CatalogVerb::Check {
            name: Some(name.clone()),
        },
        _ => return Err("usage: catalog diff NAME | catalog check [NAME]".into()),
    }))
}
