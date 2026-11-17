//! The pin written with a splice ([`super::pin`]): each gate the spliced
//! closures name that no earlier pin names, at its definition digest now;
//! on the run's first pin, the required file's digest and lock list.
use super::Loaded;
use super::pin::{self, Pin, RequiredPin};
use super::splice::Planned;
use crate::error::Result;
use crate::write::Op;
use crate::{runs, time};
use sha2::{Digest, Sha256};
use std::path::Path;

/// A gate's definition digest now: a gate definition's digest; for a
/// delivery, the mode file's digest (or `builtin:<mode>`) followed by each
/// mode gate's digest. `None` when it does not load.
pub fn definition_now(root: &Path, top: Option<&Path>, gate: &str) -> Option<String> {
    if let Some(name) = crate::deliver::step::posted_of(gate) {
        return Some(format!("posted:{name}"));
    }
    if let Some(name) = crate::deliver::step::mode_of(gate) {
        let mode = crate::deliver::mode::mode(root, name).ok()?;
        let own = match std::fs::read(root.join(&mode.source)) {
            Ok(bytes) => format!("sha256:{}", crate::index::hex(&Sha256::digest(&bytes))),
            Err(_) => format!("builtin:{name}"),
        };
        let gates: Vec<&str> = mode.gates.iter().map(|d| d.digest.as_str()).collect();
        return Some(
            std::iter::once(own.as_str())
                .chain(gates)
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    crate::gate::def::find_repository(root, top, gate)
        .ok()
        .map(|d| d.digest)
}

/// The required file as the first pin records it.
fn required(root: &Path, loaded: &Loaded) -> Result<RequiredPin> {
    let repo = (loaded.subject.as_ref()).map(|s| (s.top.as_path(), s.name.as_str()));
    let r = super::required::find(root, repo)?;
    Ok(RequiredPin {
        source: r.source.clone().unwrap_or_else(|| {
            format!(
                "{}/{}.toml",
                super::template::DIR,
                super::template::RESERVED
            )
        }),
        digest: r.digest.clone().unwrap_or_else(|| "absent".into()),
        unskippable: r.unskippable,
    })
}

/// The pin op to write before `covers` (the splice files' names), or
/// `None` when a pin left by an interrupted write is reused.
pub(super) fn op(
    root: &Path,
    loaded: &Loaded,
    planned: &[Planned],
    covers: Vec<String>,
    now: i64,
) -> Result<Option<Op>> {
    let run_dir = runs::dir(&loaded.rec);
    let (pins, _) = pin::read(root, &run_dir, &loaded.rec.id);
    let top = loaded.subject.as_ref().map(|s| s.top.as_path());
    // A leftover of an interrupted write is compared, not counted as earlier.
    let prior = match pin::leftover(root, &pins) {
        true => &pins[..pins.len() - 1],
        false => &pins[..],
    };
    let earlier: Vec<&str> = (prior.iter())
        .flat_map(|p| p.gates.iter().map(|(g, _)| g.as_str()))
        .collect();
    let mut gates: Vec<(String, String)> = Vec::new();
    let named = (planned.iter())
        .flat_map(|p| p.closure.all())
        .flat_map(|t| t.steps.iter())
        .flat_map(|s| s.gates.iter());
    for gate in named {
        if earlier.contains(&gate.as_str()) || gates.iter().any(|(g, _)| g == gate) {
            continue;
        }
        let now = definition_now(root, top, gate).unwrap_or_else(|| "undefined".into());
        gates.push((gate.clone(), now));
    }
    let first = prior.is_empty();
    let new = Pin {
        path: String::new(),
        seq: 0,
        kind: if loaded.graph.planned() { "use" } else { "set" }.into(),
        covers,
        required: if first {
            Some(required(root, loaded)?)
        } else {
            None
        },
        gates,
        recorded: time::stamp(now),
    };
    let taken = pin::max_seq(root, &run_dir);
    Ok(pin::next(root, &pins, new, taken)?.map(|p| {
        let bytes = pin::bytes(&loaded.rec.id, &p);
        Op::Create {
            path: pin::path(&run_dir, p.seq, &bytes),
            bytes,
        }
    }))
}
