//! The block a hook adds when a definition that applies here does not
//! parse: compact lines only (no snippet), at most five, then a count,
//! each ending with what the fault means in plain words. A hook never
//! fails over it: a broken rule is skipped, never treated as a denial.
use crate::defs::Problem;
use crate::workspace::Workspace;
use std::path::Path;
use writfold_core::diag::line;

/// The most lines the block shows.
pub const MOST: usize = 5;
pub const HEADER: &str =
    "Definitions with errors (skipped until fixed; `writfold kb lint` shows each):";
/// What an unreadable guard rule means.
pub const GUARD: &str = "This rule is not guarding.";
/// What an unreadable gate definition means.
pub const GATE: &str = "This gate cannot be run or met.";

/// A problem's first fault, compact, with its consequence.
pub fn entry(problem: &Problem, consequence: &str) -> String {
    let compact = problem
        .report
        .diagnostics
        .first()
        .map_or_else(|| problem.path.clone(), line);
    let gap = match compact.ends_with(['?', '.']) {
        true => " ",
        false => ". ",
    };
    format!("{compact}{gap}{consequence}")
}

/// The block over `lines`; empty when there are none.
pub fn block(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = format!("{HEADER}\n");
    for l in lines.iter().take(MOST) {
        out.push_str(l);
        out.push('\n');
    }
    if lines.len() > MOST {
        out.push_str(&format!("… and {} more\n", lines.len() - MOST));
    }
    out
}

/// The block for what applies at `cwd`: the guard rules and the gate
/// definitions of the KB root and of the repository around `cwd`.
pub fn here(ws: &Workspace, cwd: &Path) -> String {
    let (_, guards) = crate::guard::rules::all(&ws.root, cwd);
    let top = crate::defs::lint::enclosing(&ws.root, cwd);
    let (_, gates) = crate::gate::def::all(&ws.root, top.as_deref());
    let mut lines: Vec<String> = guards.iter().map(|p| entry(p, GUARD)).collect();
    lines.extend(gates.iter().map(|(_, p)| entry(p, GATE)));
    block(&lines)
}

/// The stderr line of a guard rule a hook skips.
pub fn skipped(problem: &Problem) -> String {
    format!("writfold kb hook: guard rule skipped: {}\n", problem.why())
}
