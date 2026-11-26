//! Pinned agent closures (#161, RFC 2026-11-26-pinned-agent-closures): a
//! delivery mode that names an agent (`agent = "<id>"`) brings an entry
//! `agent:<id>` into the run's pins, whose digest is that agent's closure
//! digest (the one `agent inspect` prints and an invocation records). It
//! is compared as a delivery word is: while a step owing such a mode is
//! open, against the closure now; once the step is done, not at all (the
//! done step's own `!stale` covers it). The comparison is the proved
//! `policy::pin_one`; this module names the entries and finds the steps
//! and digests they stand for. Tested, not proved.
use std::path::Path;

/// The prefix of an agent entry.
pub const PREFIX: &str = "agent:";

/// The agent a delivery word's mode names, when it names one.
pub fn agent_of(root: &Path, word: &str) -> Option<String> {
    let mode = crate::deliver::step::mode_of(word)?;
    crate::deliver::mode::mode(root, mode).ok()?.agent
}

/// The closure digest of agent `id` now, over the KB root and the run's
/// repository; `None` when it does not resolve.
pub fn closure_now(root: &Path, top: Option<&Path>, id: &str) -> Option<String> {
    let t = crate::agents::inspect::table_at(root, top);
    crate::agents::inspect::decide(&t, id).1.ok()
}

/// Whether `word` is an agent entry.
pub fn is_agent_word(word: &str) -> bool {
    word.starts_with(PREFIX)
}

/// Whether a step owing `gates` names entry `word`: a gate or delivery
/// word by itself, an agent entry through a mode that names the agent.
pub fn names(root: &Path, gates: &[String], word: &str) -> bool {
    match word.strip_prefix(PREFIX) {
        Some(id) => (gates.iter()).any(|g| agent_of(root, g).as_deref() == Some(id)),
        None => gates.iter().any(|g| g == word),
    }
}

/// `words`, each followed by the agent entry its mode names, each once.
pub fn with_agents<'a>(root: &Path, words: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in words {
        let agent = agent_of(root, word).map(|id| format!("{PREFIX}{id}"));
        for w in std::iter::once(word.clone()).chain(agent) {
            if !out.contains(&w) {
                out.push(w);
            }
        }
    }
    out
}
