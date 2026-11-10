use super::{Refusal, content_digest};
use writfold_domain::ContentDigest;
use writfold_domain::citation::Coverage;

const KEY_BYTES: usize = 1024;
const KIND_BYTES: usize = 64;
const NEEDLE_BYTES: usize = 4096;
const PAGE_LIMIT: usize = 1024;

fn bounded(value: String, limit: usize, what: &'static str) -> Result<String, Refusal> {
    if value.is_empty() || value.len() > limit || value.contains('\0') {
        return Err(Refusal::InvalidRequest(what));
    }
    Ok(value)
}

/// Exact record key in the backend's key space (a Dolt node ID, a graph node ID).
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct RecordKey(String);

impl RecordKey {
    pub fn new(value: String) -> Result<Self, Refusal> {
        bounded(
            value,
            KEY_BYTES,
            "record key must be 1..=1024 bytes without NUL",
        )
        .map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Record kind; navigation data, not an inference rule.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RecordKind(String);

impl RecordKind {
    pub fn new(value: String) -> Result<Self, Refusal> {
        bounded(
            value,
            KIND_BYTES,
            "record kind must be 1..=64 bytes without NUL",
        )
        .map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Named conditional head (a branch or graph head), never a pin by itself.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct HeadName(String);

impl HeadName {
    pub fn new(value: String) -> Result<Self, Refusal> {
        bounded(
            value,
            KEY_BYTES,
            "head name must be 1..=1024 bytes without NUL",
        )
        .map(Self)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact read of one record, optionally bound to an already-held digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRead {
    key: RecordKey,
    expected: Option<ContentDigest>,
}

impl ExactRead {
    pub fn new(key: RecordKey, expected: Option<ContentDigest>) -> Self {
        Self { key, expected }
    }
    pub fn key(&self) -> &RecordKey {
        &self.key
    }
    pub fn expected(&self) -> Option<&ContentDigest> {
        self.expected.as_ref()
    }
}

/// Retained bytes at an exact pin. `digest` is SHA-256 of `content`, recomputed
/// by the boundary, never copied from a backend-supplied field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRecord<P> {
    pin: P,
    key: RecordKey,
    kind: RecordKind,
    content: Vec<u8>,
    digest: ContentDigest,
}

impl<P> ExactRecord<P> {
    /// The digest is computed here from the bytes, never taken from the backend.
    pub fn new(
        pin: P,
        key: RecordKey,
        kind: RecordKind,
        content: Vec<u8>,
    ) -> Result<Self, Refusal> {
        let digest = content_digest(&content)?;
        Ok(Self {
            pin,
            key,
            kind,
            content,
            digest,
        })
    }
    pub fn pin(&self) -> &P {
        &self.pin
    }
    pub fn key(&self) -> &RecordKey {
        &self.key
    }
    pub fn kind(&self) -> &RecordKind {
        &self.kind
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookupField {
    Key,
    Kind,
    Content,
}

/// `Exact` is byte equality; `Prefix`/`Substring` are byte-wise, case-sensitive
/// lexical matches. `Fuzzy` and `Semantic` exist only so a backend can refuse
/// them by type; no B0 backend implements them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchMode {
    Exact,
    Prefix,
    Substring,
    Fuzzy,
    Semantic,
}

/// `Key` is binary key order. `Ranked` is refused until a backend admits ranking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Order {
    Key,
    Ranked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupRequest {
    field: LookupField,
    mode: MatchMode,
    order: Order,
    needle: String,
    limit: usize,
}

impl LookupRequest {
    pub fn new(
        field: LookupField,
        mode: MatchMode,
        order: Order,
        needle: String,
        limit: usize,
    ) -> Result<Self, Refusal> {
        let needle = bounded(
            needle,
            NEEDLE_BYTES,
            "needle must be 1..=4096 bytes without NUL",
        )?;
        if limit == 0 || limit > PAGE_LIMIT {
            return Err(Refusal::InvalidRequest("page limit must be 1..=1024"));
        }
        Ok(Self {
            field,
            mode,
            order,
            needle,
            limit,
        })
    }
    pub fn field(&self) -> LookupField {
        self.field
    }
    pub fn mode(&self) -> MatchMode {
        self.mode
    }
    pub fn order(&self) -> Order {
        self.order
    }
    pub fn needle(&self) -> &str {
        &self.needle
    }
    pub fn limit(&self) -> usize {
        self.limit
    }
}

/// A lookup hit names a record; `read_exact` expands it. It is not a session
/// `CandidateHandle`: the application allocates handles over these keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupHit {
    key: RecordKey,
    kind: RecordKind,
    digest: ContentDigest,
}

impl LookupHit {
    pub fn new(key: RecordKey, kind: RecordKind, content: &[u8]) -> Result<Self, Refusal> {
        Ok(Self {
            key,
            kind,
            digest: content_digest(content)?,
        })
    }
    pub fn key(&self) -> &RecordKey {
        &self.key
    }
    pub fn kind(&self) -> &RecordKind {
        &self.kind
    }
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
}

/// Continuation bound to the exact pin and request. It is not a capability:
/// the backend recomputes the scan position and coverage from `after`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupCursor<P> {
    pin: P,
    request: LookupRequest,
    after: RecordKey,
}

impl<P> LookupCursor<P> {
    pub fn new(pin: P, request: LookupRequest, after: RecordKey) -> Self {
        Self {
            pin,
            request,
            after,
        }
    }
    pub fn pin(&self) -> &P {
        &self.pin
    }
    pub fn request(&self) -> &LookupRequest {
        &self.request
    }
    pub fn after(&self) -> &RecordKey {
        &self.after
    }
}

/// `coverage` is search coverage measured by scanning, kept separate from
/// presentation completeness (`next`). Zero hits mean absence only when the
/// coverage is complete.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupPage<P> {
    pin: P,
    hits: Vec<LookupHit>,
    coverage: Coverage,
    next: Option<LookupCursor<P>>,
}

impl<P> LookupPage<P> {
    pub fn new(
        pin: P,
        hits: Vec<LookupHit>,
        coverage: Coverage,
        next: Option<LookupCursor<P>>,
    ) -> Self {
        Self {
            pin,
            hits,
            coverage,
            next,
        }
    }
    pub fn pin(&self) -> &P {
        &self.pin
    }
    pub fn hits(&self) -> &[LookupHit] {
        &self.hits
    }
    pub fn coverage(&self) -> Coverage {
        self.coverage
    }
    pub fn next(&self) -> Option<&LookupCursor<P>> {
        self.next.as_ref()
    }
}
