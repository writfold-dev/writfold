//! Descriptor-relative effects for a qualified local Linux filesystem.
//!
//! No path resolution follows symlinks. Parent descriptors remain pinned, including
//! when directories are renamed. Cooperating writers must retain lock entries and
//! use the same lock protocol. Metadata comparisons are not lock-free ABA proofs.

mod error;
mod identity;
mod path;
mod read;
mod root;

pub use error::{BudgetKind, FsError, PathError, WriteError};
pub use identity::FileIdentity;
pub use path::RelativePath;
pub use read::{OpenedFile, ReadExtent, ReadResult};
pub use root::{ContainedRoot, PinnedFile};
