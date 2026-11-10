//! The `writfold trial run` citation template. This is a Writfold-side
//! convenience, not Weft syntax: inside the Weft source given to `trial run`,
//! `citation "tuft:@VIEW#N"` names the N-th row (1-based) of the Tuft view
//! `@VIEW`, in the projected order before the emit budget pages it. Writfold
//! replaces the whole string literal with that row's exact retained binding
//! digest, so Weft reads ordinary `citation "<digest>"` source.
//!
//! Every `"tuft:` in the source must be such a placeholder, directly after the
//! `citation` keyword, and must resolve to a retained row; anything else
//! refuses before Weft runs.
use crate::trial::Result;
use tuft::view::Row;

const OPEN: &str = "\"tuft:";

/// One substituted placeholder: the template text and what it resolved to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub placeholder: String,
    pub binding: String,
    pub path: String,
    pub line: u64,
}

/// A Tuft view's name (without `@`) and its projected rows.
pub type View<'a> = (&'a str, &'a [Row]);

fn ident(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// `citation` followed by at least one separator ends `before`.
fn after_citation(before: &str) -> bool {
    let trimmed = before.trim_end_matches([' ', '\t', '\r', '\n']);
    trimmed.len() < before.len()
        && trimmed.strip_suffix("citation").is_some_and(|rest| {
            !rest
                .bytes()
                .next_back()
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}

/// Parse `@VIEW#N` (the text between `tuft:` and the closing quote).
fn parse(body: &str) -> Option<(&str, usize)> {
    let (view, index) = body.strip_prefix('@')?.split_once('#')?;
    let valid = ident(view)
        && !index.is_empty()
        && !index.starts_with('0')
        && index.bytes().all(|b| b.is_ascii_digit());
    Some((view, index.parse().ok().filter(|_| valid)?))
}

fn resolve(placeholder: &str, body: &str, view: Option<View<'_>>) -> Result<Resolved> {
    let (name, index) = parse(body).ok_or_else(|| {
        format!("unresolved placeholder {placeholder}: expected \"tuft:@VIEW#N\" with N from 1")
    })?;
    let (view_name, rows) = view.ok_or_else(|| {
        format!("unresolved placeholder {placeholder}: no --tuft program was given")
    })?;
    if name != view_name {
        return Err(format!(
            "unresolved placeholder {placeholder}: the Tuft program's view is @{view_name}"
        )
        .into());
    }
    let row = rows.get(index - 1).ok_or_else(|| {
        format!(
            "unresolved placeholder {placeholder}: @{view_name} has {} rows",
            rows.len()
        )
    })?;
    let evidence = row.evidence.as_ref().ok_or_else(|| {
        format!("unresolved placeholder {placeholder}: @{view_name} does not `retain` its rows")
    })?;
    if !evidence
        .binding
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'.' | b'-' | b'_'))
    {
        return Err(format!("binding {:?} is not a plain digest", evidence.binding).into());
    }
    Ok(Resolved {
        placeholder: placeholder.to_owned(),
        binding: evidence.binding.clone(),
        path: row.path.clone(),
        line: row.line,
    })
}

/// Substitute every placeholder in `source`, refusing any that does not
/// resolve. Returns the exact source Weft reads.
pub fn substitute(source: &str, view: Option<View<'_>>) -> Result<(String, Vec<Resolved>)> {
    let mut out = String::with_capacity(source.len());
    let mut resolved = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find(OPEN) {
        let (before, tail) = rest.split_at(at);
        let body_and_rest = &tail[OPEN.len()..];
        let end = body_and_rest
            .find(['"', '\n'])
            .filter(|&end| body_and_rest[end..].starts_with('"'))
            .ok_or("unterminated \"tuft:...\" placeholder")?;
        let body = &body_and_rest[..end];
        let placeholder = format!("{OPEN}{body}\"");
        out.push_str(before);
        if !after_citation(&out) {
            return Err(format!(
                "unresolved placeholder {placeholder}: it must directly follow `citation`"
            )
            .into());
        }
        let found = resolve(&placeholder, body, view)?;
        out.push('"');
        out.push_str(&found.binding);
        out.push('"');
        resolved.push(found);
        rest = &body_and_rest[end + 1..];
    }
    out.push_str(rest);
    Ok((out, resolved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tuft::view::Evidence;

    fn row(binding: Option<&str>) -> Row {
        Row {
            path: "src/a.rs".into(),
            range: (0, 3),
            line: 2,
            evidence: binding.map(|b| Evidence {
                binding: b.into(),
                bytes: b"abc".to_vec(),
            }),
        }
    }

    #[test]
    fn placeholders_resolve_to_the_exact_binding() -> Result<()> {
        let rows = [row(Some("sha256:aa")), row(Some("sha256:bb"))];
        let source = "asset a = citation \"tuft:@v#2\";\nasset b = citation\n  \"tuft:@v#1\";";
        let (out, resolved) = substitute(source, Some(("v", &rows)))?;
        assert_eq!(
            out,
            "asset a = citation \"sha256:bb\";\nasset b = citation\n  \"sha256:aa\";"
        );
        assert_eq!(resolved.len(), 2);
        assert_eq!(substitute("no templates", None)?.0, "no templates");
        Ok(())
    }

    #[test]
    fn anything_unresolved_refuses() {
        let rows = [row(Some("sha256:aa")), row(None)];
        let view = Some(("v", &rows[..]));
        for source in [
            "asset a = citation \"tuft:@v#3\";",
            "asset a = citation \"tuft:@v#0\";",
            "asset a = citation \"tuft:@v#01\";",
            "asset a = citation \"tuft:@w#1\";",
            "asset a = citation \"tuft:@v#2\";",
            "asset a = citation \"tuft:v#1\";",
            "asset a = citation \"tuft:@v#1",
            "asset a = xcitation \"tuft:@v#1\";",
            "asset a = citation\"tuft:@v#1\";",
            "// see \"tuft:@v#1\"",
        ] {
            assert!(substitute(source, view).is_err(), "{source}");
        }
        assert!(substitute("asset a = citation \"tuft:@v#1\";", None).is_err());
    }
}
