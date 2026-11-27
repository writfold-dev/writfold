//! `repo:<store>/<glob>` scope entries (gate-evidence spec, stage 6): a
//! gate's scope may name paths in a declared store (`.writfold/stores/`),
//! so a receipt, cover row or statement about it stops being current when
//! those paths move there, as for paths in the subject.
//!
//! The store's clone on this machine is found as citations find it
//! ([`crate::stale::repos::resolve_in`]) and read at its target commit
//! (`origin/HEAD`, else `HEAD`): committed bytes only, never its work
//! tree, never fetched. Its in-scope paths join the scope digest
//! ([`super::subject::Snapshot::scoped`]). A store that is not declared,
//! not found here, a partial clone, or unreadable makes the digest empty,
//! and empty present content is never the same content
//! ([`writfold_core::gate::evidence`], proved): such a gate is not met, and
//! nothing about it is recorded (`gate run`, `gate cover`, `gate attest`
//! refuse).
use crate::stale::repos;
use crate::workspace::{CANCELLED, Workspace, budgets};
use std::collections::{BTreeMap, BTreeSet};
use writfold_runtime::git::{GitExecutable, gate_ls_tree};

/// What a store's entries read: its tree at its target commit, or `None`
/// when it cannot be read here.
pub type Trees = BTreeMap<String, Option<Vec<(String, String)>>>;

/// `[!]repo:<store>/<glob>` as `(store, [!]glob)`.
pub fn entry(pattern: &str) -> Option<(&str, String)> {
    let (not, bare) = match pattern.strip_prefix('!') {
        Some(rest) => ("!", rest),
        None => ("", pattern),
    };
    let (store, glob) = bare.strip_prefix("repo:")?.split_once('/')?;
    Some((store, format!("{not}{glob}")))
}

/// Whether a `repo:` entry is well formed: a store name and a relative glob.
pub fn well_formed(pattern: &str) -> bool {
    match entry(pattern) {
        Some((store, glob)) => {
            let bare = glob.strip_prefix('!').unwrap_or(&glob);
            crate::stale::kinds::is_word(store)
                && !bare.is_empty()
                && !bare.starts_with('/')
                && !bare.contains('\\')
        }
        None => !pattern.trim_start_matches('!').starts_with("repo:"),
    }
}

/// The subject's own patterns, and each store's.
pub fn split<'a>(patterns: &[&'a str]) -> (Vec<&'a str>, BTreeMap<&'a str, Vec<String>>) {
    let mut local = Vec::new();
    let mut stores: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for p in patterns {
        match entry(p) {
            Some((store, glob)) => stores.entry(store).or_default().push(glob),
            None => local.push(*p),
        }
    }
    (local, stores)
}

/// The stores any of `scopes` names.
pub fn named<'a>(scopes: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
    (scopes.into_iter())
        .filter_map(|p| entry(p).map(|(s, _)| s.to_owned()))
        .collect()
}

/// One store's tree at its target commit, if it can be read here.
fn tree(git: &GitExecutable<'_>, ws: &Workspace, name: &str) -> Option<Vec<(String, String)>> {
    let stores = crate::store::read(&ws.root).ok()?;
    let url = stores.get(name)?.url.as_str();
    let dir = repos::resolve_in(Some(ws), name, Some(url))?;
    let git_dir = repos::git_dir(&dir)?;
    if repos::partial(&git_dir) {
        return None;
    }
    let declared = stores.get(name)?;
    let commit = crate::declared::store_commit(git, &git_dir, declared).ok()??;
    let entries = gate_ls_tree(git, &git_dir, &commit, budgets().ok()?, &CANCELLED).ok()?;
    Some(entries.into_iter().map(|e| (e.path, e.id)).collect())
}

/// Read every store in `names`.
pub fn read(git: &GitExecutable<'_>, ws: &Workspace, names: &BTreeSet<String>) -> Trees {
    (names.iter())
        .map(|name| (name.clone(), tree(git, ws, name)))
        .collect()
}

/// Refuse to record anything about a scope whose stores cannot be read
/// here (its digest is empty).
pub fn readable(digest: &str, patterns: &[&str]) -> crate::error::Result<()> {
    if !digest.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = named(patterns.iter().copied()).into_iter().collect();
    Err(crate::error::KbError::Refused(format!(
        "this gate's scope names repo:{}, which cannot be read here: declare it in .writfold/stores/ and clone it (writfold kb repos)",
        names.join(", repo:")
    )))
}

#[cfg(test)]
mod tests {
    use super::{entry, split, well_formed};

    #[test]
    fn entries_name_a_store_and_a_glob() {
        assert_eq!(entry("repo:docs/adr/**"), Some(("docs", "adr/**".into())));
        assert_eq!(
            entry("!repo:docs/adr/old/**"),
            Some(("docs", "!adr/old/**".into()))
        );
        assert_eq!(entry("src/**"), None);
        assert!(well_formed("repo:docs/adr/**") && well_formed("src/**"));
        for bad in ["repo:docs", "repo:Docs!/x", "repo:docs//x", "repo:docs/"] {
            assert!(!well_formed(bad), "{bad}");
        }
        let (local, stores) = split(&["src/**", "repo:docs/adr/**", "!repo:docs/adr/x.md"]);
        assert_eq!(local, ["src/**"]);
        assert_eq!(stores["docs"], ["adr/**", "!adr/x.md"]);
    }
}
