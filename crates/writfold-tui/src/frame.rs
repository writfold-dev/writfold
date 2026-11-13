//! Full view contents and the clock-free plain frame.
use crate::app::{App, View};
use crate::lines::{self, Line, Tone, line};
use crate::source::field;
use crate::{progress, side};

pub struct Content {
    pub title: String,
    pub lines: Vec<Line>,
    pub current: Option<usize>,
}

pub fn detail(
    app: &App,
    view: View,
    width: Option<u16>,
    expanded: bool,
    positions: bool,
) -> Content {
    let scope = if matches!(view, View::Stale | View::Brief) {
        app.project().unwrap_or_else(|| "(inferred)".into())
    } else {
        app.run().unwrap_or_default()
    };
    let mut content = Content {
        title: format!("{} {scope}", view.name()),
        lines: Vec::new(),
        current: None,
    };
    let Some(answer) = app.answer(view) else {
        content.lines.push(line("(no run selected)", Tone::Dim));
        return content;
    };
    match view {
        View::Progress => {
            (content.lines, content.current) =
                progress::content(answer, width, expanded, positions);
            if answer.trailer_text("reason").as_deref() == Some("no-plan")
                && let Some(gates) = app.answer(View::Gates)
            {
                content.lines.extend(side::gates(gates));
            }
        }
        View::Gates => content.lines = side::gates(answer),
        View::Inbox => content.lines = side::inbox(answer),
        _ => content.lines = lines::reply(answer),
    }
    content
}

pub fn header(app: &App, narrow: bool) -> String {
    let Some(row) = app.row() else {
        return "writfold watch — no run selected".into();
    };
    if narrow {
        return format!(
            "run {}/{} {} [{}]",
            app.selected + 1,
            app.order.len(),
            field(row, "id"),
            field(row, "status")
        );
    }
    let mut text = format!(
        "{} [{}]{}  {}",
        field(row, "id"),
        field(row, "status"),
        lines::suffix(row, false),
        field(row, "text")
    );
    if let Some(project) = app.project() {
        text.push_str(&format!("  project={project}"));
    }
    match &app.stamps.subject {
        Ok(Some(subject)) => text.push_str(&format!("  subject={}", subject.name)),
        Ok(None) => text.push_str("  subject=none"),
        Err(error) => text.push_str(&format!("  subject=error: {error}")),
    }
    text
}

pub fn footer(app: &App, interactive: bool) -> String {
    if interactive && let Some(notice) = &app.notice {
        return format!("theme={}  {notice}", app.look.label());
    }
    let at = |key| {
        app.runs
            .trailer_text(key)
            .unwrap_or_else(|| "unknown".into())
    };
    let mut out = format!(
        "kb={} index={}  {}",
        at("kb"),
        at("index"),
        crate::poll::subject_text(&app.stamps.subject)
    );
    if interactive {
        out.push_str(&format!(
            "  asked {}  theme={}  read-only; no daemon  ? help  q quit",
            app.asked,
            app.look.label()
        ));
    } else {
        out.push_str("  read-only; no daemon, nothing here claims a run is live");
    }
    out
}

pub fn tabs(app: &App, narrow: bool) -> String {
    if narrow {
        let index = View::ALL
            .iter()
            .position(|v| *v == app.view)
            .map_or(1, |i| i + 1);
        return format!("{} {index}/{}", app.view.name(), View::ALL.len());
    }
    View::ALL
        .iter()
        .map(|view| {
            let tab = format!("{} {}", view.digit(), view.name());
            if *view == app.view {
                format!("[{tab}]")
            } else {
                tab
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn help() -> Vec<Line> {
    let keys = [
        "writfold watch: a read-only view of the KB verbs",
        "j k ↑ ↓: select run / scroll main pane; [ ]: previous / next run",
        "Enter: switch focus; Esc: runs; PgUp PgDn: scroll main pane",
        "Tab / Shift-Tab: next / previous view; 0: runs (narrow)",
        "1/p progress; 2/g gates; 3/t timeline; 4/i inbox",
        "6/s stale; 7/b brief; 8/e resume",
        "x: expand / collapse Progress; o: open runs; P: selected project",
        "r: ask again; ?: help (any key closes); q / Ctrl-C: quit",
        "T: next theme (24-bit colour only); the choice is not written.",
        "c is unbound; no key writes. Inbox prints confirm: and accept: commands.",
    ];
    let verbs = [
        "Runs: writfold kb runs --plan [--open] [--project P] --json",
        "Progress: writfold kb plan status --run R --gates --json",
        "Gates: writfold kb gate status --run R --json",
        "Timeline: writfold kb timeline R --json",
        "Inbox / Resume: writfold kb resume R --json",
        "Stale: writfold kb stale P --cached --json",
        "Brief: writfold kb brief P --json",
        "KB and local state are polled every 2 s; the subject every 10 s.",
        "--repo selects the gate subject. No checker is started; no daemon.",
        "Theme: --theme, else WRITFOLD_THEME, else theme in config.toml, else hone.",
        "Colour: --color auto|truecolor|ansi|none; NO_COLOR / TERM=dumb disable auto colour.",
    ];
    keys.into_iter()
        .map(|s| line(s, Tone::Key))
        .chain(verbs.into_iter().map(|s| line(s, Tone::Plain)))
        .chain(crate::theme::ThemeId::ALL.into_iter().map(|theme| {
            line(
                format!("{}: {}", theme.name(), theme.description()),
                Tone::Plain,
            )
        }))
        .collect()
}

fn runs(app: &App, out: &mut String) {
    let open = app
        .runs
        .rows
        .iter()
        .filter(|r| field(r, "status") != "closed")
        .count();
    out.push_str(&format!(
        "== runs ({} listed, {open} not closed) ==\n",
        app.order.len()
    ));
    if app.runs.error.is_some() {
        append(out, lines::reply(&app.runs));
        return;
    }
    for (n, i) in app.order.iter().enumerate() {
        for (k, item) in lines::run_item(&app.runs.rows[*i]).into_iter().enumerate() {
            out.push_str(if k == 0 && n == app.selected {
                "> "
            } else {
                "  "
            });
            out.push_str(&item.text);
            out.push('\n');
        }
    }
    append(out, lines::notes(&app.runs));
}

fn append(out: &mut String, lines: impl IntoIterator<Item = Line>) {
    for line in lines {
        out.push_str(&line.text);
        out.push('\n');
    }
}

pub fn once(app: &mut App, only: Option<View>) -> String {
    let mut out = String::new();
    runs(app, &mut out);
    let views = only.map_or_else(
        || vec![View::Progress, View::Gates, View::Inbox, View::Timeline],
        |view| vec![view],
    );
    for view in views {
        app.load_view(view);
        if view == View::Progress || view == View::Resume {
            app.load_view(View::Gates);
        }
        let content = detail(app, view, None, true, true);
        out.push_str(&format!("== {} ==\n", content.title));
        if view == View::Resume
            && let Some(gates) = app.answer(View::Gates)
        {
            append(&mut out, [side::gate_summary(gates)]);
        }
        append(&mut out, content.lines);
    }
    out.push_str(&format!("== {} ==\n", footer(app, false)));
    out
}
