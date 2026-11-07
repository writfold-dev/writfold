//! IO-free descriptors only: neither identities nor observations authorize effects.
mod effect_id;
mod observation;
mod prepared;
mod records;
mod references;
pub use effect_id::EffectId;
pub use observation::*;
pub use prepared::*;
pub use records::*;
pub use references::{BindingRevision, RepositoryId, WorktreeId};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidWorktree;
impl fmt::Display for InvalidWorktree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid worktree descriptor")
    }
}
impl std::error::Error for InvalidWorktree {}
pub type Result<T, E = InvalidWorktree> = std::result::Result<T, E>;
mod sealed {
    pub trait Sealed {}
}
/// Digest construction carries no authentication or native-outcome authority.
pub trait WorktreeDigest: sealed::Sealed + Copy + Eq {
    const KIND: &'static str;
    fn from_digest(bytes: [u8; 32]) -> Self;
    fn as_digest(&self) -> &[u8; 32];
}
macro_rules! identities {
    ($($name:ident, $kind:literal;)+) => {$(
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name([u8; 32]);
        impl sealed::Sealed for $name {}
        impl WorktreeDigest for $name {
            const KIND: &'static str = $kind;
            fn from_digest(bytes: [u8; 32]) -> Self { Self(bytes) }
            fn as_digest(&self) -> &[u8; 32] { &self.0 }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!("wfwt1:", $kind, ":sha256:"))?;
                for byte in self.0 { write!(f, "{byte:02x}")?; }
                Ok(())
            }
        }
        impl FromStr for $name {
            type Err = InvalidWorktree;
            fn from_str(text: &str) -> Result<Self> {
                let hex = text.strip_prefix(concat!("wfwt1:", $kind, ":sha256:")).ok_or(InvalidWorktree)?;
                Ok(Self(parse_hash(hex)?))
            }
        }
    )+};
}
identities! {
    RecipeDigest, "recipe";
    CaptureDigest, "capture";
    CandidateDigest, "candidate";
    PreparedEditDigest, "prepared-edit";
    RequestKeyDigest, "request-key";
    ObservationDigest, "observation";
}
pub fn parse_hash(text: &str) -> Result<[u8; 32]> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(InvalidWorktree);
    }
    let mut result = [0; 32];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|_| InvalidWorktree)?;
    }
    Ok(result)
}
pub(super) fn label(text: &str) -> Result<()> {
    if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control) {
        Err(InvalidWorktree)
    } else {
        Ok(())
    }
}
