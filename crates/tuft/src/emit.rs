//! `emit`: a deterministic compact view plus a mandatory envelope.
//!
//! The budget covers the `columns` line and the row lines; each line costs
//! `ceil((bytes + 1) / 4)` estimated tokens (its LF included). Rows stop at a
//! row boundary. Rows not shown are counted and a cursor resumes them; the
//! envelope is always printed and is outside the budget.
use crate::ir::Plan;
use crate::render::{coverage_json, coverage_text, quote, row_json, row_text};
use crate::view::{Projected, SearchReport, Stop};
use crate::{VERSION, digest_hex};
use std::fmt::Write as _;

pub const ESTIMATE: &str = "ceil((bytes+1)/4) per text line";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Text,
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

/// Render the view from `cursor` (or the first row) in `format`.
pub fn emit(run: Run<'_>, cursor: Option<&str>, format: Format) -> Result<String, CursorError> {
    let fields = &run.plan.view.fields;
    let lines: Vec<String> = run
        .projected
        .rows
        .iter()
        .map(|r| row_text(r, fields))
        .collect();
    let view_digest = digest_hex(lines.join("\n").as_bytes());
    let tag = cursor_tag(run.plan, &view_digest);
    let from = start(cursor, &tag, lines.len())?;
    let columns = format!(
        "columns {}",
        fields
            .iter()
            .map(|f| f.name())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let budget = run.plan.emit.budget_tokens;
    let mut used = estimate(&columns);
    let mut shown = 0;
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
    Ok(match format {
        Format::Text => text(run, &page, &columns, &lines),
        Format::Json => json(run, &page),
    })
}

struct Page {
    from: usize,
    shown: usize,
    omitted: usize,
    used: usize,
    cursor: Option<String>,
    /// Cost of the next row when not even one row fits the budget.
    blocked: Option<usize>,
}

fn cursor_tag(plan: &Plan, view_digest: &str) -> String {
    let material = format!(
        "tuft-cursor/0\n{}\n{}\n{}\n{view_digest}",
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

fn stop_text(stop: Option<Stop>) -> &'static str {
    match stop {
        Some(Stop::HitLimit) => " stopped=hit-limit",
        None => "",
    }
}

fn text(run: Run<'_>, page: &Page, columns: &str, lines: &[String]) -> String {
    let plan = run.plan;
    let find = &plan.find;
    let (matcher, pattern) = find.matcher.describe();
    let mut out = format!(
        "{VERSION} program=sha256:{} commit={}\n",
        plan.program, find.commit
    );
    let _ = writeln!(
        out,
        "find @{} {matcher} {} in {} limit={}: {} hits={}{}",
        find.binding,
        quote(pattern),
        quote(&find.scope),
        find.budget.positions(),
        coverage_text(run.search.coverage),
        run.search.hits,
        stop_text(run.search.stop)
    );
    if let (Some(retain), Some(count)) = (&plan.retain, run.retained) {
        let around = retain.around.map_or("range=match".to_owned(), |n| {
            format!("range=lines(around={n})")
        });
        let _ = writeln!(
            out,
            "retain @{} {around} bindings={count} schema=writfold-citation/1",
            retain.binding
        );
    }
    let _ = writeln!(
        out,
        "view @{} rows={} merged={} limited={} from={} shown={} omitted={} budget_tokens={} used={} estimate={ESTIMATE}",
        plan.view.binding,
        run.projected.rows.len(),
        run.projected.merged,
        run.projected.limited,
        page.from,
        page.shown,
        page.omitted,
        plan.emit.budget_tokens,
        page.used
    );
    if let Some(cost) = page.blocked {
        let _ = writeln!(
            out,
            "notice next row needs {cost} tokens; raise budget_tokens"
        );
    }
    let _ = writeln!(out, "cursor={}", page.cursor.as_deref().unwrap_or("none"));
    out.push_str(columns);
    for line in lines.iter().skip(page.from).take(page.shown) {
        out.push('\n');
        out.push_str(line);
    }
    out
}

fn json(run: Run<'_>, page: &Page) -> String {
    let plan = run.plan;
    let find = &plan.find;
    let (matcher, pattern) = find.matcher.describe();
    let stop = run
        .search
        .stop
        .map_or("null".to_owned(), |_| quote("hit-limit"));
    let retain = match (&plan.retain, run.retained) {
        (Some(r), Some(count)) => format!(
            "{{\"binding\":{},\"around_lines\":{},\"bindings\":{count},\"schema\":\"writfold-citation/1\"}}",
            quote(&r.binding),
            r.around.map_or("null".to_owned(), |n| n.to_string())
        ),
        _ => "null".to_owned(),
    };
    let columns: Vec<String> = plan.view.fields.iter().map(|f| quote(f.name())).collect();
    let rows: Vec<String> = run
        .projected
        .rows
        .iter()
        .skip(page.from)
        .take(page.shown)
        .map(|row| row_json(row, &plan.view.fields))
        .collect();
    format!(
        "{{\"tuft\":{},\"program\":\"sha256:{}\",\"commit\":{},\
\"find\":{{\"binding\":{},\"matcher\":{},\"pattern\":{},\"scope\":{},\"limit\":{},\"coverage\":{},\"stopped\":{stop},\"hits\":{}}},\
\"retain\":{retain},\
\"view\":{{\"binding\":{},\"columns\":[{}],\"rows\":{},\"merged\":{},\"limited\":{},\"from\":{},\"shown\":{},\"omitted\":{}}},\
\"budget\":{{\"tokens\":{},\"used\":{},\"estimate\":{}}},\"cursor\":{},\"rows\":[{}]}}",
        quote(VERSION),
        plan.program,
        quote(&find.commit),
        quote(&find.binding),
        quote(matcher),
        quote(pattern),
        quote(&find.scope),
        find.budget.positions(),
        coverage_json(run.search.coverage),
        run.search.hits,
        quote(&plan.view.binding),
        columns.join(","),
        run.projected.rows.len(),
        run.projected.merged,
        run.projected.limited,
        page.from,
        page.shown,
        page.omitted,
        plan.emit.budget_tokens,
        page.used,
        quote(ESTIMATE),
        page.cursor.as_deref().map_or("null".to_owned(), quote),
        rows.join(",")
    )
}
