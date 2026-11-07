use super::{InvalidWorktree, Result};
use std::{fmt, str::FromStr};

/// Logical host effect identity, not a content hash and never an authority.
/// Exact ASCII token spelling: [A-Za-z0-9][A-Za-z0-9_.:-]{0,255}.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EffectId(String);
impl EffectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}
impl TryFrom<String> for EffectId {
    type Error = InvalidWorktree;
    fn try_from(value: String) -> Result<Self> {
        if value.len() > 256 || !crate::identity_grammar::token(&value) {
            return Err(InvalidWorktree);
        }
        Ok(Self(value))
    }
}
impl FromStr for EffectId {
    type Err = InvalidWorktree;
    fn from_str(value: &str) -> Result<Self> {
        if value.len() > 256 || !crate::identity_grammar::token(value) {
            return Err(InvalidWorktree);
        }
        Ok(Self(value.to_owned()))
    }
}
impl fmt::Display for EffectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
