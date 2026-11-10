//! The verbose text envelope (`--verbose`) and the JSON form (`--json`).
use crate::VERSION;
use crate::emit::{ESTIMATE, Page, Run};
use crate::render::{coverage_json, coverage_text, quote, row_json, row_text};
use crate::view::Stop;
use std::fmt::Write as _;

pub(crate) fn stop_text(stop: Option<Stop>) -> &'static str {
    match stop {
        Some(Stop::HitLimit) => " stopped=hit-limit",
        None => "",
    }
}

pub(crate) fn text(run: Run<'_>, page: &Page) -> String {
    let plan = run.plan;
    let find = &plan.find;
    let fields = &plan.view.fields;
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
    let names: Vec<&str> = fields.iter().map(|f| f.name()).collect();
    out.push_str("columns ");
    out.push_str(&names.join(" "));
    for row in run.projected.rows.iter().skip(page.from).take(page.shown) {
        out.push('\n');
        out.push_str(&row_text(row, fields));
    }
    out
}

pub(crate) fn json(run: Run<'_>, page: &Page) -> String {
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
