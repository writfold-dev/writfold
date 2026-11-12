//! Citation tokens and compact code locators, read with the
//! `writfold-sources` grammar. A token that cannot be read is kept with its
//! reason, so `stale` reports it instead of losing it.
use crate::model::Citation;
use serde_json::{Map, Value};
use writfold_sources::citations::{iter_cite_tokens, parse_range};

/// Every citation token in `text`, in order.
pub fn tokens(text: &str) -> Vec<Citation> {
    iter_cite_tokens(text)
        .map(|token| {
            let mut citation = Citation {
                line: u32::try_from(token.line_no()).unwrap_or(u32::MAX),
                illustrative: token.is_illustrative(),
                complete: token.complete(),
                raw: token.raw().to_owned(),
                fields: "{}".into(),
                ..Citation::default()
            };
            if citation.illustrative {
                return citation;
            }
            if !citation.complete {
                citation.malformed = Some("unterminated citation token".into());
                return citation;
            }
            match token.fields() {
                Ok(fields) => fill(&mut citation, fields.iter()),
                Err(error) => citation.malformed = Some(error.message().to_owned()),
            }
            citation
        })
        .collect()
}

fn fill<'a>(citation: &mut Citation, fields: impl Iterator<Item = (&'a str, &'a str)>) {
    let mut map = Map::new();
    for (key, value) in fields {
        map.insert(key.to_owned(), Value::String(value.to_owned()));
    }
    let get = |key: &str| map.get(key).and_then(Value::as_str).map(str::to_owned);
    citation.kind = get("kind");
    citation.repo = get("repo");
    citation.rev = get("rev");
    citation.path = get("path");
    citation.blob = get("blob");
    citation.span = get("span");
    citation.seen = get("seen");
    citation.ttl = get("ttl");
    citation.state = get("state");
    citation.marked = get("stale");
    if let Some(range) = get("L") {
        match parse_range(&range) {
            Ok(range) => {
                citation.first = range.first().to_usize().and_then(|n| u32::try_from(n).ok());
                citation.last = range.last().to_usize().and_then(|n| u32::try_from(n).ok());
            }
            Err(error) => citation.malformed = Some(error.message().to_owned()),
        }
    }
    citation.fields = Value::Object(map).to_string();
}

/// A compact locator `<repo>@<commit>:<path>[:a-b]`, read as a code citation
/// without a digest. `None` when the text is not one.
pub fn locator(text: &str, line: u32) -> Option<Citation> {
    let (repo, rest) = text.trim().split_once('@')?;
    let (rev, rest) = rest.split_once(':')?;
    if repo.is_empty() || rev.len() < 7 || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (path, range) = match rest.rsplit_once(':') {
        Some((path, range)) if parse_range(range).is_ok() => (path, Some(range)),
        _ => (rest, None),
    };
    if path.is_empty() {
        return None;
    }
    let mut fields = vec![
        ("kind", "code"),
        ("repo", repo),
        ("rev", rev),
        ("path", path),
    ];
    fields.extend(range.map(|range| ("L", range)));
    let mut citation = Citation {
        line,
        complete: true,
        raw: text.trim().to_owned(),
        ..Citation::default()
    };
    fill(&mut citation, fields.into_iter());
    Some(citation)
}

/// What a citation points at, as a reference token: `code:<repo>:<path>`,
/// `commit:<repo>@<rev>` or `pr:<repo>#<n>` (nodes outside the KB), or a
/// record ID, ticket, typed ID or `path:<file>` that Phase B resolves.
/// `None` for kinds that name nothing in the graph (web pages, queries).
pub fn target(c: &Citation) -> Option<String> {
    if c.illustrative || c.malformed.is_some() || !c.complete {
        return None;
    }
    let fields: Value = serde_json::from_str(&c.fields).ok()?;
    let get = |key: &str| {
        fields
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    match c.kind.as_deref()? {
        "code" => Some(format!(
            "code:{}:{}",
            c.repo.as_deref()?,
            c.path.as_deref()?
        )),
        "commit" => Some(format!(
            "commit:{}@{}",
            c.repo.as_deref()?,
            c.rev.as_deref()?
        )),
        "pr" => Some(format!("pr:{}#{}", c.repo.as_deref()?, get("n")?)),
        "ticket" => get("id").map(|id| super::ticket(&id).unwrap_or(id)),
        "ev" => get("id").map(|id| super::typed("ev:", &id)),
        "proposal" => get("id").map(|id| super::typed("proposal/", &id)),
        "gate" => get("proposal").map(|id| super::typed("proposal/", &id)),
        "record" => c.path.as_ref().map(|path| format!("path:{path}")),
        "deploy" => get("asset").or_else(|| get("service")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{locator, target, tokens};

    #[test]
    fn tokens_and_locators() {
        let text = "a\nsee ⟦cite kind=code repo=svc rev=abc1234 path=src/a.rs L=3-5 span=00⟧ and \
                    ⟦cite illus kind=web⟧\n⟦cite kind=web seen=2026-01-01T00:00Z";
        let found = tokens(text);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].line, 2);
        assert_eq!((found[0].first, found[0].last), (Some(3), Some(5)));
        assert_eq!(found[0].repo.as_deref(), Some("svc"));
        assert!(found[1].illustrative);
        assert!(found[2].malformed.is_some());
        let compact = locator("svc@abc1234def:src/lib.rs:10-12", 4);
        assert_eq!(
            compact.as_ref().and_then(|c| c.path.as_deref()),
            Some("src/lib.rs")
        );
        assert_eq!(compact.and_then(|c| c.last), Some(12));
        assert!(locator("not a locator", 1).is_none());
        let targets: Vec<_> = tokens(
            "⟦cite kind=ticket id=\"abc 12\"⟧ ⟦cite kind=record path=ktlo/a.md state=done⟧ \
             ⟦cite kind=web url=x⟧ ⟦cite kind=pr repo=acme/x n=4⟧",
        )
        .iter()
        .map(target)
        .collect();
        assert_eq!(
            targets,
            [
                Some("ABC-12".into()),
                Some("path:ktlo/a.md".into()),
                None,
                Some("pr:acme/x#4".into())
            ]
        );
    }
}
