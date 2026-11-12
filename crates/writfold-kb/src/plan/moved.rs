//! `!template-moved`: a template the run spliced whose file now differs
//! from the snapshot the run keeps. The run is unchanged (it folds from its
//! snapshots); the flag only says the template has moved on, as `!stale`
//! says a done step's gate has. There is no migration: to adopt the new
//! version, skip the remaining old steps and splice it again.
use super::fold::Graph;
use super::template;
use std::path::Path;

/// One spliced template that has moved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Moved {
    pub name: String,
    /// The snapshot's digest, as the run's steps carry it.
    pub digest: String,
    /// What the file is now: `@<digest>`, or `absent or unreadable`.
    pub now: String,
}

impl Moved {
    /// Whether a step from `name` at `digest` is flagged.
    pub fn covers(&self, name: &str, digest: &str) -> bool {
        self.name == name && self.digest == digest
    }

    /// The note that explains the flag.
    pub fn note(&self) -> String {
        format!(
            "template {} moved since it was spliced (snapshot @{}, now {}); the run keeps its snapshot, nothing migrates: to adopt the new version, skip the remaining old steps and splice it again",
            self.name,
            short(&self.digest),
            self.now
        )
    }
}

/// The first 12 hex digits of a `sha256:` digest.
pub fn short(digest: &str) -> &str {
    digest
        .trim_start_matches("sha256:")
        .get(..12)
        .unwrap_or_default()
}

/// Every template (name and snapshot digest) the run's steps come from
/// whose current file has another digest or is gone. A template given
/// with `--from FILE` has no current file to compare and is never moved.
pub fn find(kb: &Path, repo: Option<(&Path, &str)>, graph: &Graph) -> Vec<Moved> {
    let mut out: Vec<Moved> = Vec::new();
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for step in &graph.steps {
        let pair = (step.template.as_str(), step.digest.as_str());
        if seen.contains(&pair) || step.source.starts_with("--from") {
            continue;
        }
        seen.push(pair);
        let now = match template::find(kb, repo, &step.template) {
            Ok((t, _)) if t.digest == step.digest => continue,
            Ok((t, _)) => format!("@{}", short(&t.digest)),
            Err(_) => "absent or unreadable".to_owned(),
        };
        out.push(Moved {
            name: step.template.clone(),
            digest: step.digest.clone(),
            now,
        });
    }
    out
}
