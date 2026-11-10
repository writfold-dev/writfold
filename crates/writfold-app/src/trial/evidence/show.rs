//! `writfold trial evidence show`: the exact retained bytes of one binding,
//! verified against its descriptor before printing, with optional context
//! lines read from the pinned blob.
//!
//! Context lines are never part of the retained bytes: they print in their
//! own marked sections, each line prefixed `L| `, and the pinned blob is
//! checked first against the descriptor's whole-file digest and against the
//! retained bytes at the retained range.
use super::super::retain::read_pinned;
use super::super::{Result, Trial};
use super::{Entry, load, resolve};
use std::path::Path;
use tuft::locate::line_at;
use writfold_wire::citation::selection_digest;

/// What `show` printed, and the entry it verified.
pub struct Shown {
    pub entry: Entry,
    pub text: String,
}

/// The whole lines of `content` numbered `first..=last` (1-based), each as
/// `L| text`.
fn numbered(content: &[u8], first: u64, last: u64) -> String {
    let mut out = String::new();
    let lines = content.split_inclusive(|&b| b == b'\n');
    for (number, line) in (1u64..).zip(lines) {
        if number > last {
            break;
        }
        if number >= first {
            let text = String::from_utf8_lossy(line);
            let text = text.strip_suffix('\n').unwrap_or(&text);
            out.push_str(&format!("{number}| {text}\n"));
        }
    }
    out
}

/// Context around `entry` from the pinned blob, after checking the blob.
fn context(state: &Path, repo: &Path, entry: &Entry, n: u64) -> Result<(String, String, u64)> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let blob = read_pinned(&executable, repo, &entry.commit, &entry.path)?;
    let encoded = std::fs::read(super::dir(state).join(format!("{}.json", entry.binding)))?;
    let descriptor = writfold_wire::citation::decode_descriptor(&encoded, super::super::limits()?)?;
    if selection_digest(&blob.content)?.as_str()
        != descriptor.source().reference().content().as_str()
    {
        return Err("the pinned blob does not match the descriptor's file digest".into());
    }
    let (lo, hi) = (entry.range.0 as usize, entry.range.1 as usize);
    if blob.content.get(lo..hi) != Some(entry.bytes.as_slice()) {
        return Err(
            "the pinned blob does not hold the retained bytes at the retained range".into(),
        );
    }
    let start = line_at(&blob.content, lo);
    let end = line_at(&blob.content, hi.saturating_sub(1).max(lo));
    let before = numbered(&blob.content, start.saturating_sub(n).max(1), start - 1);
    let after = numbered(&blob.content, end + 1, end + n);
    Ok((before, after, start))
}

/// Resolve `which` (digest, label or digest prefix), verify it and render it.
pub fn show(state: &Path, which: &str, context_lines: u64, repo: Option<&Path>) -> Result<Shown> {
    let (binding, via) = resolve(state, which)?;
    let mut entry = load(state, &binding)?;
    let mut text = String::new();
    if let Some(via) = via {
        text.push_str(&format!(
            "# {via} resolved to {binding} (local; cite the digest)\n"
        ));
    }
    let around = match (context_lines, repo) {
        (0, _) => None,
        (n, Some(repo)) => Some(context(state, repo, &entry, n)?),
        (_, None) => return Err("--context-lines needs --repo DIR to read the pinned blob".into()),
    };
    if let Some((_, _, line)) = &around {
        entry.line = Some(*line);
    }
    let line = entry.line.map_or("?".to_owned(), |l| l.to_string());
    text.push_str(&format!(
        "binding {binding}\nsource {}@{} bytes {}..{} line {line}\n",
        entry.path, entry.commit, entry.range.0, entry.range.1
    ));
    if let Some((before, _, _)) = &around {
        text.push_str("--- context before (pinned blob, not retained)\n");
        text.push_str(before);
    }
    match std::str::from_utf8(&entry.bytes) {
        Ok(raw) => {
            text.push_str("--- retained (exact bytes, verified)\n");
            text.push_str(raw);
            if !raw.is_empty() && !raw.ends_with('\n') {
                text.push_str("\n--- (no final LF in the retained bytes)\n");
            }
        }
        Err(_) => {
            text.push_str("--- retained (exact bytes, verified; not UTF-8, shown as hex)\n");
            for chunk in entry.bytes.chunks(32) {
                let hex: String = chunk.iter().map(|b| format!("{b:02x}")).collect();
                text.push_str(&hex);
                text.push('\n');
            }
        }
    }
    if let Some((_, after, _)) = &around {
        text.push_str("--- context after (pinned blob, not retained)\n");
        text.push_str(after);
    }
    text.push_str("--- end");
    Ok(Shown { entry, text })
}
