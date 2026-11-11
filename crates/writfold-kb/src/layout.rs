//! Path to record kind. A KB root holds `inventory/assets/`; everything else
//! is classified by its position under the root. Dot-prefixed names (temporary
//! files, local tool state) are never inputs.

/// The kind of one input file.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Kind {
    Asset,
    Relationship,
    Dossier,
    Proposal,
    Ticket,
    Investigation,
    Signature,
    Runbook,
    Run,
    Checkpoint,
    Correction,
    Project,
    ProjectLog,
    Evidence,
    Doc,
}

const KINDS: [(Kind, &str); 15] = [
    (Kind::Asset, "asset"),
    (Kind::Relationship, "relationship"),
    (Kind::Dossier, "dossier"),
    (Kind::Proposal, "decision"),
    (Kind::Ticket, "ticket"),
    (Kind::Investigation, "investigation"),
    (Kind::Signature, "signature"),
    (Kind::Runbook, "runbook"),
    (Kind::Run, "run"),
    (Kind::Checkpoint, "checkpoint"),
    (Kind::Correction, "correction"),
    (Kind::Project, "project"),
    (Kind::ProjectLog, "log"),
    (Kind::Evidence, "evidence"),
    (Kind::Doc, "doc"),
];

impl Kind {
    pub fn as_str(self) -> &'static str {
        KINDS
            .iter()
            .find(|(kind, _)| *kind == self)
            .map_or("doc", |(_, name)| name)
    }

    pub fn parse(name: &str) -> Option<Self> {
        KINDS
            .iter()
            .find(|(_, text)| *text == name)
            .map(|(kind, _)| *kind)
    }

    /// Kinds whose file must yield a record identity; failing that, the file
    /// is unreadable rather than a plain document.
    pub fn needs_identity(self) -> bool {
        !matches!(self, Self::Checkpoint | Self::ProjectLog | Self::Doc)
    }
}

/// Top-level names that are never projects.
pub const RESERVED: [&str; 9] = [
    "inventory",
    "runs",
    "proposals",
    "tools",
    "ktlo",
    "investigations",
    "signatures",
    "runbooks",
    "queries",
];

/// The project a top-level directory names, if it is one. `corrections/`
/// holds correction records, so it is not a project either.
pub fn project_name(top: &str) -> Option<&str> {
    (!top.starts_with('.') && !RESERVED.contains(&top) && top != "corrections").then_some(top)
}

/// The kind of a root-relative `/`-separated path, or `None` when the path
/// is not an input (another extension, a dot name, or an ignored area).
pub fn classify(path: &str) -> Option<Kind> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.iter().any(|p| p.is_empty() || p.starts_with('.')) {
        return None;
    }
    let name = parts.last()?;
    let markdown = name.ends_with(".md");
    if !markdown && !name.ends_with(".json") {
        return None;
    }
    match parts.as_slice() {
        ["inventory", "assets", _] => (!markdown).then_some(Kind::Asset),
        ["inventory", "relationships", _] => (!markdown).then_some(Kind::Relationship),
        ["inventory", "dossiers", _] if markdown => Some(Kind::Dossier),
        ["inventory", "cursors", ..] => None,
        ["proposals", _, _, "proposal.md"] => Some(Kind::Proposal),
        ["proposals", _, file]
            if markdown && file.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
        {
            Some(Kind::Proposal)
        }
        ["ktlo", _, _] if markdown => Some(Kind::Ticket),
        ["investigations", _, _, "README.md"] => Some(Kind::Investigation),
        ["signatures", _] if markdown => Some(Kind::Signature),
        ["runbooks", _, _] if markdown => Some(Kind::Runbook),
        ["runs", _, "manifest.json"] => Some(Kind::Run),
        ["runs", _, "checkpoint.md"] => Some(Kind::Checkpoint),
        ["runs", _, "observations" | "receipts", ..] => None,
        ["corrections", _] if markdown => Some(Kind::Correction),
        [top, "STATUS.md"] if project_name(top).is_some() => Some(Kind::Project),
        [top, "log", _] if markdown && project_name(top).is_some() => Some(Kind::ProjectLog),
        [top, "evidence", _] if markdown && project_name(top).is_some() => Some(Kind::Evidence),
        _ => Some(Kind::Doc),
    }
}

/// The top-level directory of a nested path.
pub fn first_segment(path: &str) -> Option<&str> {
    path.split_once('/').map(|(top, _)| top)
}

/// The file name without its extension.
pub fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::{Kind, classify};

    #[test]
    fn classification() {
        let cases = [
            ("inventory/assets/ledger-api.json", Some(Kind::Asset)),
            ("inventory/assets/notes.md", None),
            (
                "inventory/relationships/a__b__c.json",
                Some(Kind::Relationship),
            ),
            ("inventory/dossiers/ledger-api.md", Some(Kind::Dossier)),
            ("inventory/cursors/x.json", None),
            (
                "proposals/decision/2026-01-02-x/proposal.md",
                Some(Kind::Proposal),
            ),
            ("proposals/decision/2026-01-02-x.md", Some(Kind::Proposal)),
            (
                "proposals/decision/2026-01-02-x/research/a.md",
                Some(Kind::Doc),
            ),
            ("ktlo/2026/abc-12-x.md", Some(Kind::Ticket)),
            (
                "investigations/2026/2026-01-02-x/README.md",
                Some(Kind::Investigation),
            ),
            (
                "investigations/2026/2026-01-02-x/evidence/e.md",
                Some(Kind::Doc),
            ),
            ("signatures/slow-fsync.md", Some(Kind::Signature)),
            ("runbooks/ops/restart.md", Some(Kind::Runbook)),
            ("runs/run-x/manifest.json", Some(Kind::Run)),
            ("runs/run-x/checkpoint.md", Some(Kind::Checkpoint)),
            ("runs/run-x/receipts/1.json", None),
            ("ledger/STATUS.md", Some(Kind::Project)),
            ("ledger/log/2026-01-02.md", Some(Kind::ProjectLog)),
            ("ledger/evidence/fsync.md", Some(Kind::Evidence)),
            ("ledger/README.md", Some(Kind::Doc)),
            ("tools/STATUS.md", Some(Kind::Doc)),
            ("ledger/.draft.md", None),
            (".kctx/policy.json", None),
            ("ledger/diagram.png", None),
        ];
        for (path, kind) in cases {
            assert_eq!(classify(path), kind, "{path}");
        }
    }
}
