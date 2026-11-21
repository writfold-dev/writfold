//! Which graph facts a refresh rewrites (#59). Edges partition by the file
//! that holds their reference (`prov_path` is part of every edge's key), and
//! a file's edges are a function of its own references and the resolution
//! keys every file shares: record IDs and paths, unique aliases, and the
//! files that lost a duplicate ID. So when the keys are unchanged, replacing
//! the edges of exactly the files whose references changed gives the same
//! edges as resolving everything (`proofs/Writfold/Graph.lean`,
//! `delta_eq_full`); when no reference changed, nothing is rewritten; when
//! a key changed, everything is, explicitly. Translated and proved; the
//! SQL that carries the plan out is tested against a full rebuild.

/// How a refresh brings the edges up to date.
pub enum Plan {
    /// No reference and no key changed: no edge row is written.
    Skip,
    /// The keys are unchanged: replace the edges of the files whose
    /// references changed, and only those.
    Delta,
    /// A key changed: resolve every reference again.
    Whole,
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

/// The plan for a refresh: `keys_same` when the resolution keys are equal
/// before and after, `changed[i]` when re-derived file `i`'s references
/// differ.
pub fn plan(keys_same: bool, changed: &[bool]) -> Plan {
    if !keys_same {
        return Plan::Whole;
    }
    if any(changed) {
        return Plan::Delta;
    }
    Plan::Skip
}

#[cfg(test)]
mod tests {
    use super::{Plan, plan};

    #[test]
    fn a_key_change_resolves_everything_and_nothing_changed_writes_nothing() {
        assert!(matches!(plan(false, &[]), Plan::Whole));
        assert!(matches!(plan(false, &[false]), Plan::Whole));
        assert!(matches!(plan(true, &[false, true]), Plan::Delta));
        assert!(matches!(plan(true, &[false, false]), Plan::Skip));
        assert!(matches!(plan(true, &[]), Plan::Skip));
    }
}
