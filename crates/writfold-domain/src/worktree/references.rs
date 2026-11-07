//! Scope references are nominal even when their spellings coincide.
//! ```compile_fail
//! use writfold_domain::worktree::{RepositoryId, WorktreeId};
//! let repo: RepositoryId = "same".parse().unwrap();
//! let worktree: WorktreeId = repo;
//! ```
//! ```compile_fail
//! use writfold_domain::{RealmId, RunId};
//! let realm: RealmId = "same".parse().unwrap();
//! let run: RunId = realm;
//! ```
use super::{InvalidWorktree, Result};
use std::{fmt, str::FromStr};

macro_rules! references {
    ($($name:ident;)+) => {$(
        /// Checked logical reference: exact token spelling, never an authority.
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str { &self.0 }
            pub fn into_string(self) -> String { self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidWorktree;
            fn try_from(value: String) -> Result<Self> {
                if value.len() > 4096 || !crate::identity_grammar::token(&value) { return Err(InvalidWorktree); }
                Ok(Self(value))
            }
        }
        impl FromStr for $name {
            type Err = InvalidWorktree;
            fn from_str(value: &str) -> Result<Self> {
                if value.len() > 4096 || !crate::identity_grammar::token(value) { return Err(InvalidWorktree); }
                Ok(Self(value.to_owned()))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
    )+};
}
references! {
    RepositoryId;
    WorktreeId;
    BindingRevision;
}
