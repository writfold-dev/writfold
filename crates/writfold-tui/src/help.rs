//! The help overlay: the marks, this frame's full stamps, then the keys
//! and verbs. The legend and stamps live here so the frame stays calm.
use crate::app::App;
use crate::cells::Glyphs;
use crate::lines::{Line, Tone, line};

/// What every mark means, in the glyph set the frame is drawn with.
pub fn marks(glyphs: Glyphs) -> Vec<Line> {
    let rows = if glyphs == Glyphs::Unicode {
        [
            "Marks   steps  ✓ done  ! done, now stale  – skipped  ▶ next step to pick up  ○ later  × invalid",
            "        gates  ● satisfied  ◆ attested/waived  ○ open  ! stale  × failed  ? unknown  · later, not judged",
            "        items  ■ supported  ◆ not applicable/attested  □ open  ! stale",
            "        ▶ is the recorded position: the first step neither done nor skipped. It is not a liveness claim.",
        ]
    } else {
        [
            "Marks   steps  [x] done  [!] done, now stale  [-] skipped  [>] next step to pick up  [ ] later  [#] invalid",
            "        gates  + satisfied  * attested/waived  o open  ! stale  x failed  ? unknown  . later, not judged",
            "        items  x supported  * not applicable/attested  . open  ! stale",
            "        [>] is the recorded position: the first step neither done nor skipped. It is not a liveness claim.",
        ]
    };
    rows.into_iter().map(|row| line(row, Tone::Plain)).collect()
}

/// The stamps the footer shortens or leaves out, in full.
pub fn this_frame(app: &App) -> Vec<Line> {
    let at = |key| {
        app.runs
            .trailer_text(key)
            .unwrap_or_else(|| "unknown".into())
    };
    let subject = crate::poll::subject_text(&app.stamps.subject).replacen('=', " ", 1);
    let colour = match app.look.depth {
        crate::style::Depth::Truecolor => {
            format!("theme {}   colour truecolor", app.look.theme.name())
        }
        _ => format!("colour {} (themes need truecolor)", app.look.label()),
    };
    vec![
        line(
            format!(
                "This frame   kb {}   index {}   {subject}",
                at("kb"),
                at("index")
            ),
            Tone::Plain,
        ),
        line(
            format!(
                "             asked {}   {colour}   read-only; no daemon",
                app.asked
            ),
            Tone::Plain,
        ),
    ]
}

/// The overlay's whole text.
pub fn overlay(app: &App) -> Vec<Line> {
    if app.mode == crate::kb_state::Mode::Kb {
        return crate::kb_help::overlay(app);
    }
    let mut out = marks(crate::map_view::glyphs(app));
    out.extend(this_frame(app));
    out.extend(crate::frame::help());
    out
}
