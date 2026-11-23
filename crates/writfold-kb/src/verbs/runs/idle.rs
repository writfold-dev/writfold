//! `runs --idle AGE`: the runs that are not closed and whose last recorded
//! write is older than `AGE`, oldest first. The last recorded write is the
//! newest of the manifest's `created`, the last checkpoint heading and
//! the UTC stamp in the name of any file under the run's gate, statement,
//! cover, observation, retrieval, edit, plan and delivery directories, and
//! the `recorded` time of each policy pin (whose name carries none):
//! tracked text only, so every clone answers the same. No liveness is
//! claimed, and no state is decided by the clock.
use super::{BOUNDARY, run_row};
use crate::emit::{Answer, More};
use crate::error::Result;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use std::path::Path;

/// The run directories whose file names carry a write's UTC stamp, and
/// what each file is.
const DIRS: [(&str, &str); 8] = [
    ("gates", "gate receipt"),
    ("statements", "statement"),
    ("covers", "cover"),
    ("observed", "observation"),
    ("retrievals", "retrieval"),
    ("edits", "edit receipt"),
    ("plan", "plan event"),
    ("deliveries", "delivery"),
];

/// The instant of a `YYYYMMDDTHHMMSSZ` stamp inside a file name.
fn compact(name: &str) -> Option<i64> {
    let bytes = name.as_bytes();
    (0..bytes.len().saturating_sub(15)).find_map(|i| {
        let s = name.get(i..i + 16)?;
        let shape = s.bytes().enumerate().all(|(k, b)| match k {
            8 => b == b'T',
            15 => b == b'Z',
            _ => b.is_ascii_digit(),
        });
        let iso = format!(
            "{}-{}-{}T{}:{}:{}Z",
            s.get(0..4)?,
            s.get(4..6)?,
            s.get(6..8)?,
            s.get(9..11)?,
            s.get(11..13)?,
            s.get(13..15)?
        );
        shape.then(|| crate::time::parse(&iso)).flatten()
    })
}

fn names(dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            names(&path, out);
        }
        out.push(entry.file_name().to_string_lossy().into_owned());
    }
}

/// Every recorded write's instant with what it was.
fn stamps(session: &Session<'_>, rec: &Rec, at: &str) -> Vec<(i64, &'static str)> {
    let mut out = Vec::new();
    if let Some(t) = rec.created.as_deref().and_then(crate::time::parse) {
        out.push((t, "created"));
    }
    if at != rec.created.as_deref().unwrap_or_default()
        && let Some(t) = crate::time::parse(at)
    {
        out.push((t, "checkpoint"));
    }
    let base = session.ws.root.join(runs::dir(rec));
    for (dir, what) in DIRS {
        let mut found = Vec::new();
        names(&base.join(dir), &mut found);
        out.extend(found.iter().filter_map(|n| compact(n)).map(|t| (t, what)));
    }
    out.extend(pins(&base.join(crate::plan::pin::DIR)).map(|t| (t, "policy pin")));
    out
}

/// The `recorded` instant of every policy pin: pin names carry no time.
fn pins(dir: &Path) -> impl Iterator<Item = i64> {
    let files = std::fs::read_dir(dir).into_iter().flatten().flatten();
    files.filter_map(|entry| {
        let text = std::fs::read_to_string(entry.path()).ok()?;
        let pin: serde_json::Value = serde_json::from_str(&text).ok()?;
        crate::time::parse(pin.get("recorded")?.as_str()?)
    })
}

/// `7d`, `36h`, `90m` or `45s` in seconds.
pub fn age(text: &str) -> Option<i64> {
    let unit = match text.chars().last()? {
        'd' => 86_400,
        'h' => 3_600,
        'm' => 60,
        's' => 1,
        _ => return None,
    };
    let n: i64 = text.get(..text.len() - 1)?.parse().ok()?;
    (n >= 0).then(|| n.checked_mul(unit)).flatten()
}

fn shown(seconds: i64) -> String {
    match seconds {
        s if s >= 86_400 => format!("{}d", s / 86_400),
        s if s >= 3_600 => format!("{}h", s / 3_600),
        s if s >= 60 => format!("{}m", s / 60),
        s => format!("{s}s"),
    }
}

/// The idle runs, oldest write first.
pub fn idle(
    session: &Session<'_>,
    (age, age_text): (i64, &str),
    project: Option<&str>,
    plan: bool,
) -> Result<Answer> {
    let mut answer = Answer::default();
    let current = runs::current_of(session);
    let mut found = Vec::new();
    for (rec, at) in runs::all(session)? {
        let state = runs::state(session, &rec);
        let left_out = writfold_core::runs::shown(runs::classify(&state), true)
            == writfold_core::runs::Shown::Hidden;
        if left_out || project.is_some_and(|p| rec.project.as_deref() != Some(p)) {
            continue;
        }
        let writes = stamps(session, &rec, &at);
        let instants: Vec<i64> = writes.iter().map(|(t, _)| *t).collect();
        let Some((i, since)) = writfold_core::idle::idle(session.now, &instants, age) else {
            continue;
        };
        let (mut row, _) = run_row(session, (&rec, &at, state), current.as_deref(), plan);
        let (last, what) = writes[i];
        row.more.push(More {
            label: "idle",
            text: format!(
                "{} since {} ({what})",
                shown(since),
                crate::time::stamp(last)
            ),
            ..More::default()
        });
        found.push((last, rec.id.clone(), row));
    }
    found.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    answer.counts.push(("idle", found.len()));
    answer.rows = found.into_iter().map(|(_, _, row)| row).collect();
    answer.notes.push(format!(
        "runs not closed whose last recorded write is older than {age_text}; idle is not inactive, and nothing was closed"
    ));
    if answer.rows.is_empty() {
        answer.unknown = Some("no-record");
        answer.notes.push(BOUNDARY.into());
    }
    runs::inventory(session, &mut answer)?;
    Ok(answer)
}
