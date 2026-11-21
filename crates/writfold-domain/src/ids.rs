//! Checked identities.
use std::{error::Error, fmt, str::FromStr};

/// A value that is not a valid identity of its kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityError {
    kind: &'static str,
}

impl IdentityError {
    pub fn kind(&self) -> &str {
        self.kind
    }
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}", self.kind)
    }
}

impl Error for IdentityError {}

/// `size` lowercase hexadecimal digits.
fn lower_hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

macro_rules! identity {
    ($(#[$doc:meta])* $name:ident, $accepts:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                let accepts: fn(&str) -> bool = $accepts;
                match accepts(value.as_str()) {
                    true => Ok(Self(value)),
                    false => Err(IdentityError {
                        kind: stringify!($name),
                    }),
                }
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

identity!(
    /// A SHA-256 digest: 64 lowercase hexadecimal digits.
    ContentDigest,
    |v| lower_hex(v, 64)
);
identity!(
    /// The revision of design-v3 authored material (any non-empty text).
    DesignRevision,
    |v| !v.is_empty()
);
identity!(
    /// The revision of workflow-2 authored material (any non-empty text).
    ProposalMaterialRevision,
    |v| !v.is_empty()
);

/// A Git object ID's hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GitObjectFormat {
    Sha1,
    Sha256,
}

impl GitObjectFormat {
    fn width(self) -> usize {
        match self {
            GitObjectFormat::Sha1 => 40,
            GitObjectFormat::Sha256 => 64,
        }
    }
}

/// A full Git object ID in its format's width, lowercase hexadecimal.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GitObjectId {
    format: GitObjectFormat,
    hex: String,
}

impl GitObjectId {
    pub fn new(format: GitObjectFormat, hex: String) -> Result<Self, IdentityError> {
        match lower_hex(&hex, format.width()) {
            true => Ok(Self { format, hex }),
            false => Err(IdentityError {
                kind: "GitObjectId",
            }),
        }
    }

    pub fn format(&self) -> GitObjectFormat {
        self.format
    }

    pub fn as_str(&self) -> &str {
        self.hex.as_str()
    }

    pub fn into_string(self) -> String {
        self.hex
    }
}

impl TryFrom<String> for GitObjectId {
    type Error = IdentityError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let format = match value.len() {
            40 => GitObjectFormat::Sha1,
            64 => GitObjectFormat::Sha256,
            _ => {
                return Err(IdentityError {
                    kind: "GitObjectId",
                });
            }
        };
        Self::new(format, value)
    }
}

impl FromStr for GitObjectId {
    type Err = IdentityError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_owned())
    }
}

impl fmt::Display for GitObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
