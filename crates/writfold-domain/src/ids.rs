//! Checked identities. Parsing never trims, folds case, or normalizes Unicode.
pub mod inventory;
use crate::identity_grammar as grammar;
use std::{error::Error, fmt, str::FromStr};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityError {
    kind: &'static str,
}

impl IdentityError {
    pub closed spec fn kind_value(&self) -> &'static str { self.kind }
    pub fn kind(&self) -> (kind: &str)
        ensures kind == self.kind_value(),
    {
        self.kind
    }
}
} // verus!

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}", self.kind)
    }
}
impl Error for IdentityError {}

macro_rules! identities {
    ($grammar:ident, $language:ident; $($name:ident),+ $(,)?) => {$(
        verus! {
        #[derive(Debug, Ord, PartialOrd)]
        pub struct $name(String);
        impl View for $name {
            type V = Seq<char>;
            closed spec fn view(&self) -> Seq<char> { self.0@ }
        }
        impl DeepView for $name {
            type V = Seq<char>;
            open spec fn deep_view(&self) -> Seq<char> { self@ }
        }
        impl Clone for $name {
            #[inline]
            fn clone(&self) -> (cloned: Self)
                ensures cloned == *self,
            { Self(self.0.clone()) }
        }
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::cmp::PartialEqSpecImpl for $name {
            open spec fn obeys_eq_spec() -> bool { true }
            open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
        }
        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> (equal: bool)
                ensures equal == (self@ == other@),
            { self.0.as_str().eq(other.0.as_str()) }
        }
        impl Eq for $name {}
        impl $name {
            pub closed spec fn accepts(value: Seq<char>) -> bool {
                grammar::spec::$language(vstd::utf8::encode_utf8(value))
            }
            pub closed spec fn valid(&self) -> bool { Self::accepts(self@) }
            pub fn as_str(&self) -> (value: &str)
                ensures value@ == self@,
            { self.0.as_str() }
            pub fn into_string(self) -> (value: String)
                ensures value@ == self@,
            { self.0 }
            fn parse_checked(value: &str) -> (result: Result<Self, IdentityError>)
                ensures
                    result.is_ok() <==> Self::accepts(value@),
                    result.is_ok() ==> result.unwrap()@ == value@,
                    result.is_ok() ==> result.unwrap().valid(),
            {
                if grammar::$grammar(value) { Ok(Self(value.to_owned())) }
                else { Err(IdentityError { kind: stringify!($name) }) }
            }
        }
        #[cfg(verus_keep_ghost)]
        impl vstd::std_specs::convert::TryFromSpecImpl<String> for $name {
            open spec fn obeys_try_from_spec() -> bool { true }
            closed spec fn try_from_spec(value: String) -> Result<Self, IdentityError> {
                if Self::accepts(value@) { Ok(Self(value)) }
                else { Err(IdentityError { kind: stringify!($name) }) }
            }
        }
        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> (result: Result<Self, Self::Error>)
                ensures
                    result.is_ok() <==> Self::accepts(value@),
                    result.is_ok() ==> result.unwrap()@ == value@,
                    result.is_ok() ==> result.unwrap().valid(),
            {
                if grammar::$grammar(value.as_str()) { Ok(Self(value)) }
                else { Err(IdentityError { kind: stringify!($name) }) }
            }
        }
        }
        impl std::hash::Hash for $name {
            #[inline]
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::hash::Hash::hash(&self.0, state);
            }
        }
        impl FromStr for $name {
            type Err = IdentityError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse_checked(value)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    )+};
}

identities!(slug, slug_identity; AssetId, EvidenceId, SignatureId);
identities!(relationship, relationship_language; RelationshipId);
identities!(proposal, proposal_language; ProposalId);
identities!(investigation, investigation_language; InvestigationId);
identities!(token, token_language;
    ObservationId, ClaimId, ClaimRevision, CitationId, RuleVersion,
    OperationId, SnapshotId, DerivationId, RunId, GateId, TicketId,
    AuthorityKeyId, AttestationId, EnumerationId, CorrectionId, ReceiptId,
    PolicyId, ContextId, RealmId, EvidenceRoleId, AliasId,
    InstanceId, SessionIncarnation, CatalogRevision, SchemaRevision, KeyId,
);
identities!(token, token_language; SchemaEpochId, StorageProfileId);
identities!(graph, graph_identity; GraphId);
identities!(graph_commit, graph_commit_identity; GraphCommitId);
identities!(digest, digest_identity; SchemaDigest, QueryContractDigest, OmnigraphToolGeneration);
identities!(digest, digest_identity; PlanId, SourceCaptureId, ContentDigest, PolicyDigest, ToolDigest, PlanDigest);
identities!(digest, digest_identity;
    CitationRepresentationDigest, CitationBindingDigest, CitationCorrespondenceDigest,
);
identities!(release, release_identity; ReleaseId, GitRevision);
identities!(dolt, dolt_identity; DoltCommitId);
identities!(opaque, opaque_identity;
    CheckerId, SourceId, SourceRevision, ContentId, ProjectId, ActorId,
    ProposalMaterialRevision, TargetRevision,
    AuthoredRevision, FileRevision, DesignRevision, KbId, PolicyRevision,
);

verus! {
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

impl View for GitObjectId {
    type V = (GitObjectFormat, Seq<char>);
    closed spec fn view(&self) -> Self::V { (self.format, self.hex@) }
}

impl GitObjectId {
    pub closed spec fn accepts_any(text: Seq<char>) -> bool {
        grammar::spec::release_identity(vstd::utf8::encode_utf8(text))
    }
    pub closed spec fn accepts(format: GitObjectFormat, text: Seq<char>) -> bool {
        let bytes = vstd::utf8::encode_utf8(text);
        let width: int = match format { GitObjectFormat::Sha1 => 40, GitObjectFormat::Sha256 => 64 };
        bytes.len() == width
            && grammar::spec::range_language(bytes, 0, width, grammar::spec::Alphabet::Hex)
    }
    pub closed spec fn from_parts_spec(format: GitObjectFormat, hex: String) -> Result<Self, IdentityError> {
        if Self::accepts(format, hex@) { Ok(Self { format, hex }) }
        else { Err(IdentityError { kind: "GitObjectId" }) }
    }
    fn infer_format(length: usize) -> (format: Option<GitObjectFormat>)
        ensures format == if length == 40 { Some(GitObjectFormat::Sha1) }
            else if length == 64 { Some(GitObjectFormat::Sha256) } else { None },
    {
        match length {
            40 => Some(GitObjectFormat::Sha1),
            64 => Some(GitObjectFormat::Sha256),
            _ => None,
        }
    }
    pub fn new(format: GitObjectFormat, hex: String) -> (result: Result<Self, IdentityError>)
        ensures
            result == Self::from_parts_spec(format, hex),
            result.is_ok() <==> Self::accepts(format, hex@),
            result.is_ok() ==> result.unwrap()@ == (format, hex@),
    {
        let size = match format { GitObjectFormat::Sha1 => 40, GitObjectFormat::Sha256 => 64 };
        if grammar::lower_hex(hex.as_str(), size) { Ok(Self { format, hex }) }
        else { Err(IdentityError { kind: "GitObjectId" }) }
    }
    pub fn format(&self) -> (format: GitObjectFormat)
        ensures format == self@.0,
    { self.format }
    pub fn as_str(&self) -> (text: &str)
        ensures text@ == self@.1,
    { self.hex.as_str() }
    pub fn into_string(self) -> (text: String)
        ensures text@ == self@.1,
    { self.hex }
    fn parse_checked(value: &str) -> (result: Result<Self, IdentityError>)
        ensures
            result.is_ok() <==> grammar::spec::release_identity(value.spec_bytes()),
            result.is_ok() ==> result.unwrap()@.1 == value@,
            result.is_ok() ==> Self::accepts(result.unwrap()@.0, value@),
    {
        let Some(format) = Self::infer_format(value.len()) else {
            return Err(IdentityError { kind: "GitObjectId" });
        };
        if grammar::release(value) { Ok(Self { format, hex: value.to_owned() }) }
        else { Err(IdentityError { kind: "GitObjectId" }) }
    }
}
} // verus!

impl fmt::Display for GitObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

verus! {
#[cfg(verus_keep_ghost)]
impl vstd::std_specs::convert::TryFromSpecImpl<String> for GitObjectId {
    open spec fn obeys_try_from_spec() -> bool { true }
    closed spec fn try_from_spec(value: String) -> Result<Self, IdentityError> {
        let length = vstd::utf8::encode_utf8(value@).len();
        if length == 40 { Self::from_parts_spec(GitObjectFormat::Sha1, value) }
        else if length == 64 { Self::from_parts_spec(GitObjectFormat::Sha256, value) }
        else { Err(IdentityError { kind: "GitObjectId" }) }
    }
}
impl TryFrom<String> for GitObjectId {
    type Error = IdentityError;
    fn try_from(value: String) -> (result: Result<Self, Self::Error>)
        ensures
            result.is_ok() <==> Self::accepts_any(value@),
            result.is_ok() ==> result.unwrap()@.1 == value@,
            result.is_ok() ==> Self::accepts(result.unwrap()@.0, value@),
    {
        let input: &str = &value;
        let Some(format) = Self::infer_format(input.len()) else {
            return Err(IdentityError { kind: "GitObjectId" });
        };
        Self::new(format, value)
    }
}
} // verus!

impl FromStr for GitObjectId {
    type Err = IdentityError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_checked(value)
    }
}

identities!(token, token_language; DoltDatabaseId, GraphNodeKind);
identities!(opaque, opaque_identity; GraphNodeId, GraphEdgeId);
identities!(dolt, dolt_identity; DoltIntegrationCommitId);
identities!(digest, digest_identity; DoltToolGeneration);

// Structural ledger names use the existing token grammar, not origin evidence.
identities!(token, token_language; StreamId, LedgerRecordId);
