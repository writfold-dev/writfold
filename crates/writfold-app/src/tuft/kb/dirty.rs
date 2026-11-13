//! Dirty code tokens describe the bytes searched, never a fictitious Git blob.
use crate::trial::Result;
use crate::tuft::Executed;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_kb::stale::repos;
use writfold_kb::workspace::{GitTool, Workspace};
use writfold_kb::write::cite::{self, Fields, Span, Spanned};
use writfold_kb::write::retrieval::{self, Cited};
use writfold_sources::citations::normalize_span;

pub(super) fn cite_spans(
    executed: &Executed,
    ws: Option<&Workspace>,
    dir: &Path,
    wanted: &[Span],
    now: i64,
) -> Result<Cited> {
    let commit = &executed.plan.find.commit;
    if executed.dirty_files.is_empty() {
        return Ok(retrieval::cite_spans(ws, dir, commit, wanted, now)?);
    }
    let tool = GitTool::locate()?;
    let git = tool.admit()?;
    let git_dir = repos::git_dir(dir).ok_or("repository has no Git directory")?;
    let url = cite::url(&git, &git_dir, dir)?;
    let (repo, resolvable) = repos::name_for(ws, dir, &url);
    let clean: Vec<Span> = wanted
        .iter()
        .filter(|s| !executed.dirty_files.contains_key(&s.path))
        .cloned()
        .collect();
    let mut pinned = cite::spans(&git, &git_dir, commit, &clean)?.into_iter();
    let seen = cite::seen(now);
    let mut digests = BTreeMap::new();
    let mut tokens = Vec::with_capacity(wanted.len());
    for want in wanted {
        let (spanned, dirty) = if let Some(bytes) = executed.dirty_files.get(&want.path) {
            let digest = digests
                .entry(want.path.as_str())
                .or_insert_with(|| tuft::digest_hex(bytes));
            (dirty_span(want, bytes, digest)?, Some(digest.as_str()))
        } else {
            (pinned.next().ok_or("missing pinned span")?, None)
        };
        tokens.push(cite::code_token(&Fields {
            repo: &repo,
            rev: commit,
            spanned: &spanned,
            sym: None,
            seen: &seen,
            ttl: None,
            url: &url,
            dirty,
        })?);
    }
    Ok(Cited {
        repo,
        resolvable,
        tokens,
    })
}

fn dirty_span(want: &Span, bytes: &[u8], digest: &str) -> Result<Spanned> {
    let (first, last) = writfold_kb::write::span::line_range(bytes, want.lines);
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    let selected = first
        .checked_sub(1)
        .and_then(|lo| lines.get(lo..last))
        .ok_or("dirty citation range outside searched bytes")?;
    Ok(Spanned {
        path: want.path.clone(),
        first: u32::try_from(first)?,
        last: u32::try_from(last)?,
        blob: format!("dirty:sha256:{digest}"),
        span: normalize_span(selected),
    })
}
