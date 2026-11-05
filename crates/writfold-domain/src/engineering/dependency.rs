//! Canonical, IO-free flat dependency locks with exact pinned endpoints.
//!
//! Export ranks certify only strict descent of procedure calls, not package
//! authenticity, ABI compatibility, or execution authority. The rank guard and
//! finite-chain lemma are verified; collection normalization and resolution are
//! ordinary Rust outside that selected proof scope.

use super::{EngineeringDigest, PackagePayloadDigest};
use crate::ids::RealmId;
use std::{cmp::Ordering, error::Error, fmt, str::FromStr};
use vstd::prelude::*;

/// An empty dependency identity. No trimming or Unicode normalization is done.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityError;

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("dependency identity must not be empty")
    }
}
impl Error for IdentityError {}

macro_rules! identities {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentityError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    Err(IdentityError)
                } else {
                    Ok(Self(value))
                }
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty() {
                    Err(IdentityError)
                } else {
                    Ok(Self(value.to_owned()))
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    )+};
}

identities!(PackageId, ExportName);

verus! {
/// The closed dependency vocabulary. Only procedure invocation requires descent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyKind {
    ReadReference,
    RequireContext,
    SelectRoute,
    InvokeProcedure,
    ExecuteAdapter,
    ProduceArtifact,
}

/// The production admission guard for declared procedure-call ranks.
pub fn call_rank_allowed(kind: DependencyKind, source: u64, target: u64) -> (allowed: bool)
    ensures allowed == (kind != DependencyKind::InvokeProcedure || target < source),
{
    match kind {
        DependencyKind::InvokeProcedure => target < source,
        _ => true,
    }
}

/// A nonempty chain of descending call edges cannot return to its initial rank.
/// `end` counts edges, so even a single self-call is excluded.
pub proof fn finite_call_chain_descends(ranks: Seq<u64>, end: int)
    requires
        0 < end < ranks.len(),
        forall|i: int| 0 <= i < end ==> ranks[i + 1] < #[trigger] ranks[i],
    ensures
        ranks[end] < ranks[0],
        ranks[end] != ranks[0],
    decreases end,
{
    assert(ranks[end] < ranks[end - 1]);
    if end > 1 {
        finite_call_chain_descends(ranks, end - 1);
    }
}
}

impl DependencyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadReference => "read_reference",
            Self::RequireContext => "require_context",
            Self::SelectRoute => "select_route",
            Self::InvokeProcedure => "invoke_procedure",
            Self::ExecuteAdapter => "execute_adapter",
            Self::ProduceArtifact => "produce_artifact",
        }
    }
}

/// A spelling outside the closed dependency-kind vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KindError;

impl fmt::Display for KindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown dependency kind")
    }
}
impl Error for KindError {}

impl FromStr for DependencyKind {
    type Err = KindError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "read_reference" => Ok(Self::ReadReference),
            "require_context" => Ok(Self::RequireContext),
            "select_route" => Ok(Self::SelectRoute),
            "invoke_procedure" => Ok(Self::InvokeProcedure),
            "execute_adapter" => Ok(Self::ExecuteAdapter),
            "produce_artifact" => Ok(Self::ProduceArtifact),
            _ => Err(KindError),
        }
    }
}

/// An exact package revision, not an acquisition request or authentication claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRef {
    id: PackageId,
    payload: PackagePayloadDigest,
}

impl PackageRef {
    pub fn new(id: PackageId, payload: PackagePayloadDigest) -> Self {
        Self { id, payload }
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn payload(&self) -> &PackagePayloadDigest {
        &self.payload
    }
}

/// An export with a declared finite call rank, without authority or ABI evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Export {
    name: ExportName,
    rank: u64,
}

impl Export {
    pub fn new(name: ExportName, rank: u64) -> Self {
        Self { name, rank }
    }

    pub fn name(&self) -> &ExportName {
        &self.name
    }

    pub fn rank(&self) -> u64 {
        self.rank
    }
}

/// An unadmitted package declaration; the lock constructor checks its exports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    reference: PackageRef,
    exports: Vec<Export>,
}

impl Package {
    pub fn new(reference: PackageRef, exports: Vec<Export>) -> Self {
        Self { reference, exports }
    }

    pub fn reference(&self) -> &PackageRef {
        &self.reference
    }

    pub fn exports(&self) -> &[Export] {
        &self.exports
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    package: PackageRef,
    export: ExportName,
}

impl Endpoint {
    pub fn new(package: PackageRef, export: ExportName) -> Self {
        Self { package, export }
    }

    pub fn package(&self) -> &PackageRef {
        &self.package
    }

    pub fn export(&self) -> &ExportName {
        &self.export
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    source: Endpoint,
    kind: DependencyKind,
    target: Endpoint,
}

impl Edge {
    pub fn new(source: Endpoint, kind: DependencyKind, target: Endpoint) -> Self {
        Self {
            source,
            kind,
            target,
        }
    }

    pub fn source(&self) -> &Endpoint {
        &self.source
    }

    pub fn kind(&self) -> DependencyKind {
        self.kind
    }

    pub fn target(&self) -> &Endpoint {
        &self.target
    }
}

/// Failure to admit a complete flat dependency lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockError {
    EmptyPackages,
    ConflictingPackage,
    ConflictingExport,
    MissingPackage,
    MissingExport,
    RecursiveCall,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyPackages => "dependency lock has no packages",
            Self::ConflictingPackage => "pinned package has conflicting export declarations",
            Self::ConflictingExport => "export name has conflicting ranks",
            Self::MissingPackage => "dependency endpoint references an undeclared pinned package",
            Self::MissingExport => "dependency endpoint references an undeclared export",
            Self::RecursiveCall => "procedure invocation does not strictly decrease export rank",
        })
    }
}
impl Error for LockError {}

/// An immutable, canonically ordered set of resolved dependency declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyLock {
    realm: RealmId,
    packages: Vec<Package>,
    edges: Vec<Edge>,
}

impl DependencyLock {
    /// Normalize finite owned input, resolve all endpoints, and admit call ranks.
    /// Wire size/depth budgets belong to the decoder before these vectors exist.
    pub fn new(
        realm: RealmId,
        mut packages: Vec<Package>,
        mut edges: Vec<Edge>,
    ) -> Result<Self, LockError> {
        if packages.is_empty() {
            return Err(LockError::EmptyPackages);
        }
        for package in &mut packages {
            package.exports.sort_unstable_by(|left, right| {
                text_cmp(left.name.as_str(), right.name.as_str())
                    .then_with(|| left.rank.cmp(&right.rank))
            });
            for pair in package.exports.windows(2) {
                if pair[0].name == pair[1].name && pair[0].rank != pair[1].rank {
                    return Err(LockError::ConflictingExport);
                }
            }
            package.exports.dedup();
        }
        packages.sort_unstable_by(|left, right| package_cmp(&left.reference, &right.reference));
        for pair in packages.windows(2) {
            if pair[0].reference == pair[1].reference && pair[0].exports != pair[1].exports {
                return Err(LockError::ConflictingPackage);
            }
        }
        packages.dedup();

        edges.sort_unstable_by(edge_cmp);
        edges.dedup();
        for edge in &edges {
            let source = resolve_rank(&packages, &edge.source)?;
            let target = resolve_rank(&packages, &edge.target)?;
            if !call_rank_allowed(edge.kind, source, target) {
                return Err(LockError::RecursiveCall);
            }
        }
        Ok(Self {
            realm,
            packages,
            edges,
        })
    }

    pub fn realm(&self) -> &RealmId {
        &self.realm
    }

    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

// Canonical CBOR text order: UTF-8 byte length, then unsigned bytes. This avoids
// allocating encoded copies merely to sort set entries.
fn text_cmp(left: &str, right: &str) -> Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.as_bytes().cmp(right.as_bytes()))
}

fn package_cmp(left: &PackageRef, right: &PackageRef) -> Ordering {
    text_cmp(left.id.as_str(), right.id.as_str())
        .then_with(|| left.payload.as_digest().cmp(right.payload.as_digest()))
}

fn endpoint_cmp(left: &Endpoint, right: &Endpoint) -> Ordering {
    package_cmp(&left.package, &right.package)
        .then_with(|| text_cmp(left.export.as_str(), right.export.as_str()))
}

fn edge_cmp(left: &Edge, right: &Edge) -> Ordering {
    endpoint_cmp(&left.source, &right.source)
        .then_with(|| text_cmp(left.kind.as_str(), right.kind.as_str()))
        .then_with(|| endpoint_cmp(&left.target, &right.target))
}

// Both tables have already been sorted and checked for conflicting keys. Binary
// search keeps resolution logarithmic without cloning identities into an index.
fn resolve_rank(packages: &[Package], endpoint: &Endpoint) -> Result<u64, LockError> {
    let package_index = packages
        .binary_search_by(|package| package_cmp(&package.reference, &endpoint.package))
        .map_err(|_| LockError::MissingPackage)?;
    let package = packages
        .get(package_index)
        .ok_or(LockError::MissingPackage)?;
    let export_index = package
        .exports
        .binary_search_by(|export| text_cmp(export.name.as_str(), endpoint.export.as_str()))
        .map_err(|_| LockError::MissingExport)?;
    package
        .exports
        .get(export_index)
        .map(Export::rank)
        .ok_or(LockError::MissingExport)
}
