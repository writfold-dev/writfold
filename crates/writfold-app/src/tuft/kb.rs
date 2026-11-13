//! The KB adapter of `writfold tuft run`: citation tokens for the shown
//! rows (`--cite`) and a retrieval receipt in the current KB run.
//!
//! A token is the one `writfold kb cite make` prints for the same lines at
//! the program's pinned commit (`writfold_kb::write::retrieval::cite_spans`
//! reads the blob and digests the lines with the same code), so it checks
//! as fresh under `writfold kb stale` while the cited lines are unchanged.
//! The receipt goes through the KB write protocol. With no KB and no run,
//! and without `--cite`, nothing here runs and the output is today's.
mod dirty;
use super::Executed;
use crate::trial::Result;
use std::path::{Path, PathBuf};
use tuft::emit::{Cursor, Format};
use writfold_kb::workspace::{Workspace, discover};
use writfold_kb::write::Write;
use writfold_kb::write::cite::{Lines, Span};
use writfold_kb::write::retrieval::{self, Retrieval};
use writfold_kb::{Request, Verb};

/// What the adapter does beyond the plain run.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Print a citation token under each shown row.
    pub cite: bool,
    /// Record in this KB (a refusal or error then fails the command).
    pub kb: Option<PathBuf>,
    /// Record in this run (likewise).
    pub run: Option<String>,
    /// Without `kb` or `run`: record when a KB found from `cwd` (as
    /// `writfold kb` finds it) has an open current run; problems are
    /// reported as notes, never as failures.
    pub discover: bool,
    pub cwd: PathBuf,
    /// Seconds since the epoch; `None` reads the clock.
    pub now: Option<i64>,
}

/// The printed output and notes for standard error.
pub struct Adapted {
    pub printed: String,
    pub notes: Vec<String>,
}

/// Where a receipt goes: the KB, the run (`None`: the current one) and
/// whether the user asked for it.
struct Target {
    ws: Workspace,
    run: Option<String>,
    explicit: bool,
}

fn target(options: &Options) -> Result<Option<Target>> {
    if options.kb.is_some() || options.run.is_some() {
        let ws = discover(options.kb.as_deref(), &options.cwd)?;
        return Ok(Some(Target {
            ws,
            run: options.run.clone(),
            explicit: true,
        }));
    }
    if !options.discover {
        return Ok(None);
    }
    let Ok(ws) = discover(None, &options.cwd) else {
        return Ok(None);
    };
    Ok(retrieval::current(&ws).map(|run| Target {
        ws,
        run: Some(run),
        explicit: false,
    }))
}

/// The value of `name=` in a trailer.
fn trailer_count(trailer: &str, name: &str) -> usize {
    trailer
        .split(' ')
        .find_map(|part| part.strip_prefix(name)?.strip_prefix('='))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Apply the options to one executed run.
pub fn adapt(
    executed: &Executed,
    repo: &Path,
    cursor: Option<&Cursor>,
    options: &Options,
) -> Result<Adapted> {
    let emitted = &executed.emitted;
    let mut adapted = Adapted {
        printed: emitted.printed(),
        notes: Vec::new(),
    };
    let target = target(options)?;
    if !options.cite && target.is_none() {
        return Ok(adapted);
    }
    let from = cursor.map_or(0, Cursor::offset);
    let shown = trailer_count(&emitted.trailer, "shown");
    let spans: Vec<Span> = executed
        .projected
        .rows
        .iter()
        .skip(from)
        .take(shown)
        .map(|row| Span {
            path: row.path.clone(),
            lines: Lines::Bytes(row.range.0, row.range.1),
        })
        .collect();
    let now = options.now.unwrap_or_else(writfold_kb::time::now);
    let commit = &executed.plan.find.commit;
    let discovered = (target.is_none() && options.discover)
        .then(|| discover(None, &options.cwd).ok())
        .flatten();
    let ws = target.as_ref().map(|t| &t.ws).or(discovered.as_ref());
    let cited = match dirty::cite_spans(executed, ws, repo, &spans, now) {
        Ok(cited) => cited,
        Err(error) if !options.cite && target.as_ref().is_some_and(|t| !t.explicit) => {
            adapted.notes.push(format!("not recorded: {error}"));
            return Ok(adapted);
        }
        Err(error) => return Err(error),
    };
    if !cited.resolvable && !cited.tokens.is_empty() {
        adapted.notes.push(format!(
            "repo={} is not in the KB's repository map (<git dir>/writfold/repos); `writfold kb stale` reports these citations unverifiable until it is",
            cited.repo
        ));
    }
    if options.cite {
        adapted.printed = with_cites(emitted.format, &adapted.printed, &cited.tokens);
    }
    if let Some(target) = target {
        let mut cites: Vec<String> = Vec::new();
        for token in &cited.tokens {
            if !cites.contains(token) {
                cites.push(token.clone());
            }
        }
        let receipt = Retrieval {
            run: target.run.clone(),
            tool: tuft::VERSION.to_owned(),
            program: format!("sha256:{}", executed.plan.program),
            repo: cited.repo,
            commit: commit.clone(),
            trailer: emitted.trailer.clone(),
            rows: executed.projected.rows.len(),
            shown,
            cites,
        };
        match record(&target, receipt, options) {
            Ok(note) => adapted.notes.push(note),
            Err(error) if !target.explicit => {
                adapted.notes.push(format!("not recorded: {error}"));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(adapted)
}

fn record(target: &Target, receipt: Retrieval, options: &Options) -> Result<String> {
    let request = Request {
        verb: Verb::Write(Write::Retrieval(receipt)),
        kb: Some(target.ws.root.clone()),
        cwd: options.cwd.clone(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: options.now,
        probe: writfold_kb::write::no_probe,
    };
    let output = writfold_kb::run(&request)?;
    let path = output
        .text
        .split_whitespace()
        .skip_while(|word| *word != "recorded")
        .nth(1)
        .unwrap_or("a receipt");
    Ok(format!("recorded {path} in {}", target.ws.root.display()))
}

/// The output with one `  cite: <token>` line under each shown row
/// (compact and verbose), or a `cites` array beside `rows` (JSON).
pub fn with_cites(format: Format, printed: &str, tokens: &[String]) -> String {
    if tokens.is_empty() && format != Format::Json {
        return printed.to_owned();
    }
    match format {
        Format::Json => {
            let items: Vec<String> = tokens.iter().map(|t| json_string(t)).collect();
            let body = printed.strip_suffix('}').unwrap_or(printed);
            format!("{body},\"cites\":[{}]}}", items.join(","))
        }
        Format::Compact | Format::Verbose => {
            let lines: Vec<&str> = printed.split('\n').collect();
            // Compact: the rows, then the trailer; verbose: the envelope,
            // then the rows.
            let first = match format {
                Format::Compact => 0,
                _ => lines.len().saturating_sub(tokens.len()),
            };
            let mut out = Vec::with_capacity(lines.len() + tokens.len());
            for (index, line) in lines.iter().enumerate() {
                out.push((*line).to_owned());
                if let Some(token) = index.checked_sub(first).and_then(|i| tokens.get(i)) {
                    out.push(format!("  cite: {token}"));
                }
            }
            out.join("\n")
        }
    }
}

/// A JSON string literal.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
