//! Writer ownership and write authority for the Dolt adapter.
//!
//! A writer owns exactly one realm identity, a `DoltTarget`: realm, KB,
//! database, schema epoch, storage profile and producing tool. A pin belongs to
//! it only if every one of those fields is equal; nothing is converted or cast.
//! Under `writfold-dolt-integration/1` a writer must also hold a strictly
//! verified signed Dolt approval (`SuppliedDoltAuthorization`) whose base pin is
//! the request's expected-old head and whose target is the writer's realm.
use crate::backend::Refusal;
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{PlanDigest, ReleaseId};
use writfold_wire::dolt_operation::DoltTarget;
use writfold_wire::signatures::dolt::SuppliedDoltAuthorization;

/// Identity of one writer claimant, recorded in the claim and in every commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterId(String);

impl WriterId {
    /// 1..=64 bytes of `[A-Za-z0-9._-]`.
    pub fn new(value: String) -> Result<Self, Refusal> {
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if !valid {
            return Err(Refusal::InvalidRequest(
                "writer id must be 1..=64 bytes of [A-Za-z0-9._-]",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Write authority derived only from a strictly verified signed Dolt approval.
/// It binds one expected-old pin and one target realm; it is not publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltWriteApproval {
    plan: PlanDigest,
    release: ReleaseId,
    base: DoltReadPin,
    target: DoltTarget,
}

impl DoltWriteApproval {
    /// Bootstrap approvals name no expected-old head, and migration approvals
    /// change the epoch or database; neither authorizes a B0 conditional write.
    pub fn admit(authorization: &SuppliedDoltAuthorization<'_>) -> Result<Self, Refusal> {
        let operation = authorization.operation();
        let base = operation.base().as_ref().ok_or(Refusal::Unauthorized(
            "a bootstrap approval names no expected-old head",
        ))?;
        if operation.mapping().is_some() {
            return Err(Refusal::Unauthorized(
                "a migration approval cannot authorize a same-epoch write",
            ));
        }
        Ok(Self {
            plan: operation.id().clone(),
            release: base.release().clone(),
            base: base.pin().clone(),
            target: operation.target().clone(),
        })
    }
    pub fn plan(&self) -> &PlanDigest {
        &self.plan
    }
    /// The signed predecessor release: a publication's Git expected-old value.
    pub fn release(&self) -> &ReleaseId {
        &self.release
    }
    pub fn base(&self) -> &DoltReadPin {
        &self.base
    }
    pub fn target(&self) -> &DoltTarget {
        &self.target
    }
}

/// Every identity field of `pin` equals the owned target.
pub(super) fn owns(target: &DoltTarget, pin: &DoltReadPin) -> bool {
    pin.realm() == target.realm()
        && pin.kb() == target.kb()
        && pin.database() == target.database()
        && pin.epoch() == target.epoch()
        && pin.profile() == target.profile()
        && pin.producer() == target.producer()
}

/// The approval names exactly this expected-old pin and this writer's realm.
pub(super) fn authorizes(
    approval: Option<&DoltWriteApproval>,
    target: &DoltTarget,
    expected_old: &DoltReadPin,
) -> Result<(), Refusal> {
    let approval = approval.ok_or(Refusal::Unauthorized(
        "a verified signed Dolt approval is required",
    ))?;
    if approval.target() != target {
        return Err(Refusal::Unauthorized(
            "approval targets a different realm or epoch",
        ));
    }
    if approval.base() != expected_old {
        return Err(Refusal::Unauthorized(
            "approval names a different expected-old head",
        ));
    }
    Ok(())
}

/// The recorded claim: who holds the writer lease for which realm.
pub(super) fn claim_record(owner: &WriterId, target: &DoltTarget) -> Vec<u8> {
    format!(
        "writfold-dolt-writer/1\nowner={}\nrealm={}\nkb={}\ndatabase={}\nepoch={}\nproducer={}\n",
        owner.as_str(),
        target.realm().as_str(),
        target.kb().as_str(),
        target.database().as_str(),
        target.epoch().id().as_str(),
        target.producer().as_str()
    )
    .into_bytes()
}
