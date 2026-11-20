//! The delivery guard's question over a run's candidates, shared by the
//! PreToolUse hook and `deliver match`: may these bytes be posted by this
//! call? The decision is [`writfold_core::delivery::guard`]; this module
//! builds each candidate's [`Seen`] and words the refusal so that it names
//! the repair.
use super::found::Found;
use writfold_core::delivery::{self, Body, Seen, Unread};

/// What a guarded call posts, as the caller read it.
pub enum Read {
    Bytes(Vec<u8>),
    /// The bytes cannot be read, and why.
    Unread(String),
}

/// The call: its command words or tool name, the mode a route names, the
/// file flag that would post a checked file, and the rule for unread bytes.
pub struct Call<'a> {
    pub name: &'a str,
    pub mode: Option<&'a str>,
    pub file_flag: Option<&'a str>,
    pub unread: Unread,
    /// `deliver match`: a posted candidate still matches (the text has
    /// been posted; that is what is being confirmed).
    pub spent_matches: bool,
}

pub fn trimmed(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(0, |i| i + 1);
    &bytes[..end]
}

fn same(f: &Found, bytes: Option<&[u8]>) -> bool {
    match (bytes, f.payload.as_deref()) {
        (Some(b), Some(p)) => trimmed(p) == trimmed(b),
        _ => false,
    }
}

/// Each candidate as the kernel sees it.
pub fn seen(found: &[Found], call: &Call<'_>, bytes: Option<&[u8]>) -> Vec<Seen> {
    found
        .iter()
        .map(|f| Seen {
            same: same(f, bytes),
            ready: f.ready,
            live: f.withdrawn.is_none(),
            routed: call.mode.is_none_or(|m| f.mode.as_deref() == Some(m)),
            spent: f.posted.is_some() && !call.spent_matches,
        })
        .collect()
}

/// `None` when the call may proceed; otherwise the reason, naming the repair.
pub fn decide(
    root: &std::path::Path,
    run: &str,
    found: &[Found],
    call: &Call<'_>,
    read: &Read,
) -> Option<String> {
    let (body, bytes) = match read {
        Read::Bytes(b) => (Body::Known, Some(b.as_slice())),
        Read::Unread(_) => (Body::Unknown, None),
    };
    // `Unread` is the kernel's and is not `Copy`: rebuild it.
    let unread = if matches!(call.unread, Unread::Deny) {
        Unread::Deny
    } else {
        Unread::AllReady
    };
    if delivery::guard(body, unread, &seen(found, call, bytes)) {
        return None;
    }
    Some(match read {
        Read::Bytes(b) => known(root, run, found, call, b),
        Read::Unread(why) => match call.unread {
            Unread::Deny => unread_route(root, run, found, call, why),
            Unread::AllReady => unread_listed(run, found, why),
        },
    })
}

/// The candidate a call that went through posted (D19): the newest
/// ready, live, unposted candidate of its route whose payload is the
/// call's bytes, or, with the bytes unread, the one `ready_one` picks.
pub fn posting<'f>(found: &'f [Found], call: &Call<'_>, read: &Read) -> Option<&'f Found> {
    match read {
        Read::Bytes(b) => found.iter().rev().find(|f| {
            same(f, Some(b))
                && f.ready
                && f.withdrawn.is_none()
                && f.posted.is_none()
                && call.mode.is_none_or(|m| f.mode.as_deref() == Some(m))
        }),
        Read::Unread(_) => ready_one(found, call),
    }
}

fn mode_name(call: &Call<'_>) -> String {
    call.mode.map_or_else(|| "delivery".into(), str::to_owned)
}

/// A ready, live, unspent candidate of the call's mode.
fn ready_one<'f>(found: &'f [Found], call: &Call<'_>) -> Option<&'f Found> {
    found.iter().rev().find(|f| {
        f.ready
            && f.withdrawn.is_none()
            && f.posted.is_none()
            && call.mode.is_none_or(|m| f.mode.as_deref() == Some(m))
    })
}

fn known(
    root: &std::path::Path,
    run: &str,
    found: &[Found],
    call: &Call<'_>,
    bytes: &[u8],
) -> String {
    let digest = super::candidate::sha(bytes);
    let holders: Vec<&Found> = found.iter().filter(|f| same(f, Some(bytes))).collect();
    if let Some(f) = holders
        .iter()
        .find(|f| f.withdrawn.is_none())
        .or(holders.first())
    {
        if let Some(w) = &f.withdrawn {
            return format!(
                "candidate {} was withdrawn (by {}: {})",
                f.c.id, w.by, w.reason
            );
        }
        let mode = f.mode.as_deref().unwrap_or("invalid");
        if let Some(want) = call.mode.filter(|m| *m != mode) {
            return format!(
                "candidate {} holds these bytes but is a {mode} candidate; {} posts {want}",
                f.c.id, call.name
            );
        }
        if !f.ready {
            let next = f
                .next
                .as_ref()
                .map(|n| format!("; next: {n}"))
                .unwrap_or_default();
            return format!("candidate {} owes {}{next}", f.c.id, f.owed.join(", "));
        }
        if let Some(p) = &f.posted {
            return format!(
                "candidate {} was recorded as posted (by {}: {}). To post these bytes again, prepare a new candidate",
                f.c.id, p.by, p.reference
            );
        }
    }
    if let Some(f) = found.iter().find(|f| stale_payload_file(root, f, bytes)) {
        return format!(
            "{} is older than the candidate: writfold kb deliver check {}",
            super::render::FILE,
            f.c.id
        );
    }
    let ready = ready_one(found, call);
    let prepare = format!(
        "writfold kb deliver prepare --mode {} --from FILE",
        call.mode
            .or(ready.and_then(|f| f.mode.as_deref()))
            .unwrap_or("M")
    );
    match ready {
        Some(f) => format!(
            "these bytes ({}) are not a candidate's. The ready {} candidate is {} ({}); post that file, or prepare these bytes: {prepare}",
            short(&digest),
            f.mode.as_deref().unwrap_or("delivery"),
            f.post_path(root),
            short(
                &f.payload
                    .as_deref()
                    .map(super::candidate::sha)
                    .unwrap_or_default()
            )
        ),
        None => format!(
            "no delivery candidate of run {run} holds the bytes being posted ({}); prepare and check one: {prepare}",
            short(&digest)
        ),
    }
}

/// `sha256:5f93…`.
fn short(digest: &str) -> String {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    format!("sha256:{}…", hex.get(..4).unwrap_or(hex))
}

/// Whether `bytes` are the stored `payload.md` of `f` but not its present
/// payload (the candidate was repaired after the last check).
fn stale_payload_file(root: &std::path::Path, f: &Found, bytes: &[u8]) -> bool {
    let stored = std::fs::read(root.join(&f.c.dir).join(super::render::FILE)).ok();
    stored.is_some_and(|s| trimmed(&s) == trimmed(bytes))
        && f.payload
            .as_deref()
            .is_none_or(|p| trimmed(p) != trimmed(bytes))
}

fn unread_route(
    root: &std::path::Path,
    run: &str,
    found: &[Found],
    call: &Call<'_>,
    why: &str,
) -> String {
    let first = format!("cannot read what {} would post ({why})", call.name);
    match ready_one(found, call) {
        Some(f) => match call.file_flag {
            Some(flag) => format!(
                "{first}. Post the checked file itself: {flag} {}",
                f.post_path(root)
            ),
            None => format!(
                "{first}. Post the checked text itself: {}",
                f.post_path(root)
            ),
        },
        None => format!(
            "{first}. No {} candidate of run {run} is ready: writfold kb deliver status",
            mode_name(call)
        ),
    }
}

fn unread_listed(run: &str, found: &[Found], why: &str) -> String {
    let owing: Vec<String> = found
        .iter()
        .filter(|f| f.withdrawn.is_none() && !f.ready)
        .map(|f| format!("candidate {} owes {}", f.c.id, f.owed.join(", ")))
        .collect();
    let what = match owing.is_empty() {
        true => format!("run {run} has no live delivery candidate"),
        false => owing.join("; "),
    };
    format!("{what} ({why}, so every live candidate of the run must be ready)")
}
