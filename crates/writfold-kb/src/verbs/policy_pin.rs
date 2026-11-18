//! The current run's pin as `kb policy` shows it beside each gate.
use crate::emit::More;
use crate::plan::drift::Drift;
use crate::session::Session;
use std::path::Path;

/// The current run's drift, if a run is current.
pub struct Current {
    pub word: &'static str,
    drift: Option<Drift>,
}

/// Read the current run's pin and drift (none without a current run).
pub fn current(session: &Session<'_>, cwd: &Path) -> Current {
    let run = crate::runs::current_of(session);
    let loaded = run.and_then(|r| crate::plan::loaded(session, &r, cwd));
    let drift = loaded.map(|l| crate::plan::drift::judge(&session.ws.root, &l));
    Current {
        word: drift.as_ref().map_or("unpinned", |d| d.word),
        drift,
    }
}

impl Current {
    /// `pin: same @…/pin-1.json:1`, or `pin: moved (pinned @x, now @y)`.
    pub fn line(&self, gate: &str, now: &str) -> Option<More> {
        let drift = self.drift.as_ref()?;
        let pinned = (drift.pins.iter().rev()).find_map(|p| {
            p.gates
                .iter()
                .find(|(g, _)| g == gate)
                .map(|(_, d)| (d.clone(), p.path.clone()))
        })?;
        let short = |d: &str| {
            let d = d.strip_prefix("sha256:").unwrap_or(d);
            d.get(..12).unwrap_or(d).to_owned()
        };
        let (text, path) = match pinned.0 == now {
            true => ("same".to_owned(), Some(pinned.1)),
            false => (
                format!("moved (pinned @{}, now @{})", short(&pinned.0), short(now)),
                None,
            ),
        };
        Some(More {
            label: "pin",
            text,
            line: path.as_ref().map(|_| 1),
            path,
            ..More::default()
        })
    }
}
