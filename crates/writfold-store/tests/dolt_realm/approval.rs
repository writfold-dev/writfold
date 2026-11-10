//! Real signed Dolt approvals for the disposable writer test. They reuse the
//! public synthetic operation, policy and PUBLIC-TEST-ONLY key from
//! `writfold-wire/fixtures`, re-based on each request's expected-old pin, signed
//! and strictly verified through the production functions.
use std::path::{Path, PathBuf};
use writfold_domain::ReleaseId;
use writfold_domain::dolt::DoltReadPin;
use writfold_store::backend::Refusal;
use writfold_store::backend::dolt::DoltWriteApproval;
use writfold_wire::Limits;
use writfold_wire::dolt_operation::{DoltBase, DoltOperation, DoltTarget};
use writfold_wire::operation::Operation;
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::dolt::verify_dolt_current;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Signer {
    operation: Operation,
    release: ReleaseId,
    policy: AuthorityPolicy,
    key: ApprovalSigningKey,
    target: DoltTarget,
    limits: Limits,
}

fn fixture(directory: &str, name: &str) -> Result<Vec<u8>> {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../writfold-wire/fixtures");
    Ok(std::fs::read(root.join(directory).join(name))?)
}

impl Signer {
    /// The `selection` vector's workflow operation and base release, signing
    /// for `target`.
    pub fn load(target: DoltTarget) -> Result<Self> {
        let limits = Limits::new(1_000_000, 128, 10_000, 100_000)?;
        let selection = DoltOperation::decode(
            &fixture("dolt-operation", "selection.operation.json")?,
            limits,
        )?;
        let release = selection
            .base()
            .as_ref()
            .ok_or("selection vector has no base")?
            .release()
            .clone();
        let pem = fixture("signatures", "PUBLIC-TEST-ONLY.pem")?;
        Ok(Self {
            operation: selection.operation().clone(),
            release,
            policy: AuthorityPolicy::historical(
                &fixture("signatures", "v3-admission.policy.json")?,
                limits,
            )?,
            key: ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&pem)?)?,
            target,
            limits,
        })
    }

    /// A strictly verified approval whose base is `expected_old`.
    pub fn approve(&self, expected_old: &DoltReadPin) -> Result<DoltWriteApproval> {
        let base = DoltBase::new(self.release.clone(), expected_old.clone());
        let operation = DoltOperation::prepare(
            self.operation.clone(),
            Some(base),
            self.target.clone(),
            None,
            self.limits,
        )?;
        let toolchain = operation.operation().header().toolchain();
        let approval = self.key.sign_dolt(
            &operation,
            &self.policy,
            operation.operation().actor()?,
            toolchain,
            self.target.realm(),
        )?;
        let supplied = verify_dolt_current(
            &operation,
            &approval,
            &self.policy,
            toolchain,
            self.target.realm(),
        )?;
        Ok(DoltWriteApproval::admit(&supplied)?)
    }

    /// The approval for a request, as the conformance suite's authority.
    pub fn authority(
        &self,
        expected_old: &DoltReadPin,
    ) -> std::result::Result<DoltWriteApproval, Refusal> {
        self.approve(expected_old)
            .map_err(|_| Refusal::Unauthorized("the synthetic approval does not bind this pin"))
    }

    /// The `bootstrap` vector, verified: it names no expected-old head.
    pub fn bootstrap_admission(&self) -> Result<std::result::Result<DoltWriteApproval, Refusal>> {
        let operation = DoltOperation::decode(
            &fixture("dolt-operation", "bootstrap.operation.json")?,
            self.limits,
        )?;
        let toolchain = operation.operation().header().toolchain();
        let approval = self.key.sign_dolt(
            &operation,
            &self.policy,
            operation.operation().actor()?,
            toolchain,
            operation.target().realm(),
        )?;
        let supplied = verify_dolt_current(
            &operation,
            &approval,
            &self.policy,
            toolchain,
            operation.target().realm(),
        )?;
        Ok(DoltWriteApproval::admit(&supplied))
    }
}
