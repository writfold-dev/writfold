//! Ephemeral transport labels and bounded, byte-preserving stdio framing.
//! No corpus identity, authorization, JSON parsing or dispatch is provided here.
mod frame;
mod identity;

pub use frame::{FrameReadError, FrameReader, InputFrame};
pub use identity::{IdentityError, InvocationIdentity};
