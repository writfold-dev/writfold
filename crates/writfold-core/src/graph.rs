//! Which graph facts a refresh rewrites (#59, #65). Edges partition by the
//! file that holds their reference (`prov_path` is part of every edge's
//! key), and a file's edges are a function of its own references, whether
//! it lost a duplicate ID, and how each of its tokens resolves against the
//! keys every file shares (record IDs and paths, unique aliases). So a file
//! is affected when its references changed, its duplicate standing changed,
//! or one of its tokens now resolves differently; replacing the edges of
//! exactly the affected files gives the same edges as resolving everything
//! (`proofs/Writfold/Graph.lean`, `delta_eq_full`); when no file is
//! affected, nothing is rewritten. A real high-fanout rename may still
//! rewrite many edges: valid edges are never capped. Translated and proved;
//! the SQL that finds the moved tokens and carries the plan out is tested
//! against a full rebuild.

/// How a refresh brings the edges up to date.
pub enum Plan {
    /// No file is affected: no edge row is written.
    Skip,
    /// Replace the edges of the affected files, and only those.
    Delta,
}

/// Whether one file's edges must be resolved again: its references
/// changed, it gained or lost a duplicate ID, or one of its tokens now
/// resolves differently (#65).
pub fn affected(refs_changed: bool, dup_changed: bool, token_moved: bool) -> bool {
    refs_changed || dup_changed || token_moved
}

/// Whether any element of `changed` is true.
pub fn any(changed: &[bool]) -> bool {
    let mut index = 0;
    while index < changed.len() {
        if changed[index] {
            return true;
        }
        index += 1;
    }
    false
}

/// The plan for a refresh: `affected[i]` when file `i` is [`affected`].
pub fn plan(affected: &[bool]) -> Plan {
    if any(affected) {
        return Plan::Delta;
    }
    Plan::Skip
}

#[cfg(test)]
mod tests {
    use super::{Plan, affected, plan};

    #[test]
    fn any_affected_file_is_a_delta_and_none_writes_nothing() {
        assert!(matches!(plan(&[false, true]), Plan::Delta));
        assert!(matches!(plan(&[false, false]), Plan::Skip));
        assert!(matches!(plan(&[]), Plan::Skip));
        assert!(affected(false, false, true));
        assert!(affected(false, true, false));
        assert!(affected(true, false, false));
        assert!(!affected(false, false, false));
    }
}
