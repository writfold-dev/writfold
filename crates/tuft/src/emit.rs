//! `emit`: a deterministic view under a token budget, with an honest
//! completeness signal in every format.
//!
//! The budget covers the compact rows; each costs `ceil((bytes + 1) / 4)`
//! estimated tokens (its LF included). Rows stop at a row boundary. Rows not
//! shown are counted and a cursor resumes them. Every format pages the same
//! rows and shares cursors: the compact view (rows plus one trailer line),
//! the verbose envelope and JSON. The trailer and envelope sit outside the
//! budget and are always produced.
use crate::compact;
use crate::digest_hex;
use crate::envelope;
use crate::ir::Plan;
use crate::render::coverage_text;
use crate::view::{Projected, SearchReport};
use std::fmt::Write as _;

pub const ESTIMATE: &str = "ceil((bytes+1)/4) per compact row";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    /// `path:line: text` rows and one trailer line (the default).
    Compact,
    /// The full envelope, a `columns` line and field-per-column rows.
    Verbose,
    Json,
}

/// Inputs measured by the adapter that ran the plan.
#[derive(Clone, Copy, Debug)]
pub struct Run<'a> {
    pub plan: &'a Plan,
    pub search: SearchReport,
    /// Distinct citation bindings retained, when the plan retains.
    pub retained: Option<usize>,
    pub projected: &'a Projected,
}

/// A rendered page. `view` is the compact rows (each ending in LF), or the
/// whole verbose or JSON document; `trailer` is the compact trailer line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Emitted {
    pub format: Format,
    pub view: String,
    pub trailer: String,
}

impl Emitted {
    /// What a run prints: compact rows then the trailer, or the document.
    pub fn printed(&self) -> String {
        match self.format {
            Format::Compact => format!("{}{}", self.view, self.trailer),
            Format::Verbose | Format::Json => self.view.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CursorError {
    Malformed,
    /// Issued for a different program, commit, budget or result.
    Mismatch,
    OutOfRange,
}

impl std::fmt::Display for CursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "cursor is malformed (expected TAG.OFFSET)",
            Self::Mismatch => "cursor was issued for a different program, commit, budget or result",
            Self::OutOfRange => "cursor offset is beyond the view",
        })
    }
}

impl std::error::Error for CursorError {}

fn estimate(line: &str) -> usize {
    (line.len() + 1).div_ceil(4)
}

pub(crate) struct Page {
    pub from: usize,
    pub shown: usize,
    pub omitted: usize,
    pub used: usize,
    pub cursor: Option<String>,
    /// Cost of the next row when not even one row fits the budget.
    pub blocked: Option<usize>,
}

/// Render the view from `cursor` (or the first row) in `format`.
pub fn emit(run: Run<'_>, cursor: Option<&str>, format: Format) -> Result<Emitted, CursorError> {
    let lines_mode = run.plan.retain.as_ref().is_some_and(|r| r.around.is_some());
    let lines: Vec<String> = run
        .projected
        .rows
        .iter()
        .map(|r| compact::row(r, &run.plan.view.fields, lines_mode))
        .collect();
    let tag = cursor_tag(run.plan, &digest_hex(lines.join("\n").as_bytes()));
    let from = start(cursor, &tag, lines.len())?;
    let budget = run.plan.emit.budget_tokens;
    let (mut used, mut shown) = (0, 0);
    for line in lines.iter().skip(from) {
        let cost = estimate(line);
        if used + cost > budget {
            break;
        }
        used += cost;
        shown += 1;
    }
    let next = from + shown;
    let page = Page {
        from,
        shown,
        omitted: lines.len() - next,
        used,
        cursor: (next < lines.len()).then(|| format!("{tag}.{next}")),
        blocked: (shown == 0)
            .then(|| lines.get(from).map(|l| estimate(l)))
            .flatten(),
    };
    let trailer = trailer(run, &page);
    let view = match format {
        Format::Compact => {
            lines
                .iter()
                .skip(from)
                .take(shown)
                .fold(String::new(), |mut out, line| {
                    out.push_str(line);
                    out.push('\n');
                    out
                })
        }
        Format::Verbose => envelope::text(run, &page),
        Format::Json => envelope::json(run, &page),
    };
    Ok(Emitted {
        format,
        view,
        trailer,
    })
}

/// `# coverage=… files=E/T hits=N[ stopped=hit-limit] shown=S omitted=O
/// [limited=L ][needs=T ]cursor=C program=<12 hex>`: never without coverage
/// or cursor, the completeness signal.
fn trailer(run: Run<'_>, page: &Page) -> String {
    let mut out = format!(
        "# {} hits={}{} shown={} omitted={}",
        coverage_text(run.search.coverage),
        run.search.hits,
        envelope::stop_text(run.search.stop),
        page.shown,
        page.omitted
    );
    if run.projected.limited > 0 {
        let _ = write!(out, " limited={}", run.projected.limited);
    }
    if let Some(cost) = page.blocked {
        let _ = write!(out, " needs={cost}");
    }
    let _ = write!(
        out,
        " cursor={} program={}",
        page.cursor.as_deref().unwrap_or("none"),
        run.plan.program.get(..12).unwrap_or_default()
    );
    out
}

fn cursor_tag(plan: &Plan, view_digest: &str) -> String {
    let material = format!(
        "tuft-cursor/1\n{}\n{}\n{}\n{view_digest}",
        plan.program, plan.find.commit, plan.emit.budget_tokens
    );
    digest_hex(material.as_bytes())
        .get(..16)
        .unwrap_or_default()
        .to_owned()
}

fn start(cursor: Option<&str>, tag: &str, rows: usize) -> Result<usize, CursorError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let (given, offset) = cursor.split_once('.').ok_or(CursorError::Malformed)?;
    let offset: usize = offset.parse().map_err(|_| CursorError::Malformed)?;
    if given != tag {
        return Err(CursorError::Mismatch);
    }
    if offset >= rows {
        return Err(CursorError::OutOfRange);
    }
    Ok(offset)
}
