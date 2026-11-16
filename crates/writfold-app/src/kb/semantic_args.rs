//! Explicit semantic options; disabled builds retain unknown-option errors.
use super::args::Common;
use writfold_kb::Verb;

#[cfg(feature = "semantic")]
pub const USAGE: &str = "\n\nsemantic (explicit, never used by hooks):
  find TEXT --semantic [--semantic-ranking lexical-first|rrf]
  find --eval [FILE] --semantic [--semantic-ranking lexical-first|rrf]
  index rebuild --semantic        rebuild source index; reuse unchanged model-bound vectors
Scores and source spans are similarity pointers, never evidence or checked citations.";
#[cfg(not(feature = "semantic"))]
pub const USAGE: &str = "";

pub fn parse(
    arg: &str,
    items: &mut std::slice::Iter<'_, String>,
    common: &mut Common,
) -> Result<bool, String> {
    #[cfg(feature = "semantic")]
    match arg {
        "--semantic" => {
            common.switches.push("semantic".into());
            return Ok(true);
        }
        "--semantic-ranking" => {
            let value = items.next().ok_or("--semantic-ranking needs a value")?;
            common
                .values
                .insert("semantic-ranking".into(), value.clone());
            return Ok(true);
        }
        _ => {}
    }
    let _ = (arg, items, common);
    Ok(false)
}

pub fn allowed(verb: &str, flag: &str) -> bool {
    cfg!(feature = "semantic")
        && matches!(
            (verb, flag),
            ("find" | "index", "semantic") | ("find", "semantic-ranking")
        )
}

pub fn apply(verb: Verb, common: &Common) -> Result<Verb, String> {
    #[cfg(feature = "semantic")]
    {
        use writfold_kb::semantic::Ranking;
        let enabled = common.switches.iter().any(|s| s == "semantic");
        let rank = common.values.get("semantic-ranking");
        if rank.is_some() && !enabled {
            return Err("--semantic-ranking requires --semantic".into());
        }
        if !enabled {
            return Ok(verb);
        }
        let ranking = match rank.map(String::as_str) {
            None | Some("lexical-first") => Ranking::Lexical,
            Some("rrf") => Ranking::Rrf,
            _ => return Err("--semantic-ranking must be lexical-first or rrf".into()),
        };
        return match verb {
            Verb::Find {
                text,
                filter,
                signatures,
            } if !text.trim().is_empty() => Ok(Verb::SemanticFind {
                text,
                filter,
                signatures,
                ranking,
            }),
            Verb::FindEval { file } => Ok(Verb::SemanticEval { file, ranking }),
            Verb::IndexRebuild => Ok(Verb::SemanticRebuild),
            _ => Err("--semantic applies only to find TEXT, find --eval or index rebuild".into()),
        };
    }
    #[cfg(not(feature = "semantic"))]
    {
        let _ = common;
        Ok(verb)
    }
}

#[cfg(test)]
mod tests {
    use super::super::args::parse;
    use std::path::PathBuf;

    fn request(text: &str) -> Result<Option<writfold_kb::Request>, String> {
        parse(
            &text
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            PathBuf::from("/"),
        )
    }

    #[test]
    fn semantic_options_do_not_leak_to_other_verbs() {
        for text in [
            "why latency --semantic",
            "match latency --semantic",
            "cite make repo a 1-2 --semantic",
            "index status --semantic",
            "index verify --semantic",
            "find --kind decision --semantic",
            "find latency --semantic-ranking rrf",
            "index rebuild --semantic-ranking rrf",
            "find latency --semantic --semantic-ranking invalid",
            "find latency --semantic --semantic",
            "find latency --semantic --semantic-ranking",
        ] {
            assert!(request(text).is_err(), "{text}");
        }
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn explicit_modes_reach_typed_requests() -> Result<(), String> {
        use writfold_kb::{Verb, semantic::Ranking};
        let found = request("find timeout --semantic --kind signature --project harbor")?
            .ok_or("request")?;
        assert!(
            matches!(found.verb, Verb::SemanticFind { ranking: Ranking::Lexical, filter, .. }
            if filter.kinds == ["signature"] && filter.project.as_deref() == Some("harbor"))
        );
        let found = request("find timeout --semantic --semantic-ranking rrf")?.ok_or("request")?;
        assert!(matches!(
            found.verb,
            Verb::SemanticFind {
                ranking: Ranking::Rrf,
                ..
            }
        ));
        let eval = request("find --eval queries.jsonl --semantic --semantic-ranking rrf")?
            .ok_or("request")?;
        assert!(
            matches!(eval.verb, Verb::SemanticEval { ranking: Ranking::Rrf, file: Some(path) }
            if path == PathBuf::from("/queries.jsonl"))
        );
        assert!(matches!(
            request("index rebuild --semantic")?.ok_or("request")?.verb,
            Verb::SemanticRebuild
        ));
        assert!(matches!(
            request("find timeout")?.ok_or("request")?.verb,
            Verb::Find { .. }
        ));
        Ok(())
    }

    #[cfg(not(feature = "semantic"))]
    #[test]
    fn disabled_build_rejects_semantic_options() {
        for text in [
            "find timeout --semantic",
            "find --eval --semantic",
            "index rebuild --semantic",
        ] {
            assert!(request(text).is_err(), "{text}");
        }
    }
}
