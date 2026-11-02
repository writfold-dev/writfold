//! Checked identities. Parsing never trims, folds case, or normalizes Unicode.
use crate::identity_grammar as grammar;
use std::{error::Error, fmt, str::FromStr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityError {
    kind: &'static str,
}

impl IdentityError {
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}", self.kind)
    }
}
impl Error for IdentityError {}

macro_rules! identities {
    ($grammar:ident; $($name:ident),+ $(,)?) => {$(
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str { &self.0 }
            pub fn into_string(self) -> String { self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if grammar::$grammar(&value) { Ok(Self(value)) }
                else { Err(IdentityError { kind: stringify!($name) }) }
            }
        }
        impl FromStr for $name {
            type Err = IdentityError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if grammar::$grammar(value) { Ok(Self(value.to_owned())) }
                else { Err(IdentityError { kind: stringify!($name) }) }
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    )+};
}

identities!(slug; AssetId, EvidenceId, SignatureId);
identities!(relationship; RelationshipId);
identities!(proposal; ProposalId);
identities!(investigation; InvestigationId);
identities!(token;
    ObservationId, ClaimId, ClaimRevision, CitationId, RuleVersion,
    OperationId, SnapshotId, DerivationId, RunId, GateId, TicketId,
    AuthorityKeyId, AttestationId, EnumerationId, CorrectionId, ReceiptId,
    PolicyId, ContextId,
    KbId, InstanceId, SessionIncarnation, CatalogRevision, PolicyRevision,
    SchemaRevision, KeyId,
);
identities!(digest; PlanId, SourceCaptureId, ContentDigest, PolicyDigest, ToolDigest, PlanDigest);
identities!(release; ReleaseId, GitRevision);
identities!(dolt; DoltCommitId);
identities!(opaque;
    CheckerId, SourceId, SourceRevision, ContentId, ProjectId, ActorId,
    ProposalMaterialRevision, TargetRevision,
    AuthoredRevision, FileRevision, DesignRevision,
);

struct InventoryMembers<'a>(&'a [String]);

impl fmt::Display for InventoryMembers<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for member in self.0 {
            write!(formatter, "{}:{}", member.len(), member)?;
        }
        Ok(())
    }
}

impl ContentId {
    /// Exact nonempty inventory framing; lengths count UTF-8 bytes, not characters.
    pub fn inventory(members: &[String]) -> Self {
        Self(format!("inventory:{}", InventoryMembers(members)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GitObjectFormat {
    Sha1,
    Sha256,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GitObjectId {
    format: GitObjectFormat,
    hex: String,
}

impl GitObjectId {
    pub fn new(format: GitObjectFormat, hex: String) -> Result<Self, IdentityError> {
        let size = match format {
            GitObjectFormat::Sha1 => 40,
            GitObjectFormat::Sha256 => 64,
        };
        if grammar::lower_hex(&hex, size) {
            Ok(Self { format, hex })
        } else {
            Err(IdentityError {
                kind: "GitObjectId",
            })
        }
    }
    pub fn format(&self) -> GitObjectFormat {
        self.format
    }
    pub fn as_str(&self) -> &str {
        &self.hex
    }
    pub fn into_string(self) -> String {
        self.hex
    }
}

impl fmt::Display for GitObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
        if grammar::release(value) {
            Self::try_from(value.to_owned())
        } else {
            Err(IdentityError {
                kind: "GitObjectId",
            })
        }
    }
}
