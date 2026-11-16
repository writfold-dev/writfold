//! Knowledge-base mode's help: the marks, this frame's stamps, the keys and
//! the verb beside each view.
use crate::app::App;
use crate::cells::Glyphs;
use crate::lines::{Line, Tone, line};

pub fn marks(glyphs: Glyphs) -> Vec<Line> {
    let rows: [&str; 5] = if glyphs == Glyphs::Unicode {
        [
            "Marks   ● settled (accepted, applied, verified, done, n/a, concluded, fixed, active, closed)",
            "        ○ open (draft, proposed, todo, in-progress, blocked, open, monitoring, …)  – set aside (rejected, superseded, retired, …)",
            "        · no status, or any other word; the word itself is always shown",
            "        ! a cited source moved (stale, changed, gone, expired, past-ttl, regression, moved…)  ? unverifiable or unchecked",
            "        Marks join the entry's own flags with stale --cached rows about the same file. Runs and records meet only through checkpoint text.",
        ]
    } else {
        [
            "Marks   [x] settled (accepted, applied, verified, done, n/a, concluded, fixed, active, closed)",
            "        [ ] open (draft, proposed, todo, in-progress, blocked, open, monitoring, ...)  [-] set aside (rejected, superseded, retired, ...)",
            "        [.] no status, or any other word; the word itself is always shown",
            "        ! a cited source moved (stale, changed, gone, expired, past-ttl, regression, moved...)  ? unverifiable or unchecked",
            "        Marks join the entry's own flags with stale --cached rows about the same file. Runs and records meet only through checkpoint text.",
        ]
    };
    rows.into_iter().map(|row| line(row, Tone::Plain)).collect()
}

pub fn keys() -> Vec<Line> {
    let keys = [
        "writfold watch, knowledge base: a read-only view of the KB verbs",
        "m: back to runs; f: a listed run whose checkpoint names this record (Timeline; Progress for a run)",
        "j k ↑ ↓: select a project / record, scroll a focused inspector or the status text; [ ]: previous / next project",
        "h l / Left Right: focus projects → list → inspector; Enter cycles; Esc steps left; 0: projects (narrow)",
        "a: impact of the selected record (re-root in impact); v: in impact, direction in → out → both",
        "n N: next / previous record carrying a mark; d: calm / detailed; PgUp PgDn: page",
        "1 why; 2 status; 3 tracker; 4 impact; 5 stale; Tab / Shift-Tab: next / previous view",
        "p g t i u s b e: leave for that run view; letters never change meaning",
        "r: ask again; ?: help (any key closes); q / Ctrl-C: quit; no key writes",
    ];
    let verbs = [
        "Projects: writfold kb brief --all --json",
        "Why: writfold kb find --kind decision --project P --json, and writfold kb status project:P --json",
        "Status: writfold kb brief P --json",
        "Tracker: writfold kb impact project:P --direction in --depth 2 --json (tickets, investigations, signatures)",
        "Impact: writfold kb impact ID --direction D --depth 2 --json; the tree follows each row's edge",
        "Stale: writfold kb stale --cached --json (the whole KB); no checker is started",
        "All records: writfold kb find --kind K --json for why and tracker",
        "Inspector: writfold kb status ID, why ID (decisions), timeline ID; uses ID for code",
    ];
    keys.into_iter()
        .map(|s| line(s, Tone::Key))
        .chain(verbs.into_iter().map(|s| line(s, Tone::Plain)))
        .collect()
}

pub fn overlay(app: &App) -> Vec<Line> {
    let mut out = marks(crate::map_view::glyphs(app));
    out.extend(crate::help::this_frame(app));
    out.extend(crate::help::words());
    out.extend(keys());
    out
}
