//! Real signed Dolt approvals for disposable publication realms. The public
//! synthetic selection and bootstrap operations, policy and PUBLIC-TEST-ONLY
//! key from `writfold-wire/fixtures` are re-prepared for the realm's target
//! (and, for a selection, the actual predecessor release and base pin), signed
//! and strictly verified through the production functions. `variant` changes
//! the selection's retained-release field or the bootstrap's capture, so each
//! publication attempt is its own signed operation. Fixture bytes are only read.
use super::world::Result;
use std::path::{Path, PathBuf};
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{GitObjectId, ReleaseId};
use writfold_store::backend::Refusal;
use writfold_store::backend::dolt::DoltWriteApproval;
use writfold_wire::Limits;
use writfold_wire::dolt_operation::{DoltBase, DoltOperation, DoltTarget};
use writfold_wire::operation::{
    Operation, OperationBody, PortfolioAdmissionPlan, PortfolioReleaseSelectionPlan,
};
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::dolt::{SuppliedDoltAuthorization, verify_dolt_current};

type Admit = fn(&SuppliedDoltAuthorization<'_>) -> std::result::Result<DoltWriteApproval, Refusal>;

pub struct Signer {
    selection: PortfolioReleaseSelectionPlan,
    bootstrap: PortfolioAdmissionPlan,
    policy: AuthorityPolicy,
    key: ApprovalSigningKey,
    target: DoltTarget,
    limits: Limits,
}

fn fixture(directory: &str, name: &str) -> Result<Vec<u8>> {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../writfold-wire/fixtures");
    Ok(std::fs::read(root.join(directory).join(name))?)
}

fn vector(name: &str, limits: Limits) -> Result<OperationBody> {
    let vector = DoltOperation::decode(&fixture("dolt-operation", name)?, limits)?;
    Ok(vector.operation().body().clone())
}

impl Signer {
    pub fn load(target: DoltTarget) -> Result<Self> {
        let limits = Limits::new(1_000_000, 128, 10_000, 100_000)?;
        let OperationBody::PortfolioSelection(selection) =
            vector("selection.operation.json", limits)?
        else {
            return Err("the selection vector is not a portfolio selection".into());
        };
        let OperationBody::PortfolioAdmission(bootstrap) =
            vector("bootstrap.operation.json", limits)?
        else {
            return Err("the bootstrap vector is not a portfolio admission".into());
        };
        let pem = fixture("signatures", "PUBLIC-TEST-ONLY.pem")?;
        Ok(Self {
            selection,
            bootstrap,
            policy: AuthorityPolicy::historical(
                &fixture("signatures", "v3-admission.policy.json")?,
                limits,
            )?,
            key: ApprovalSigningKey::from_pkcs8_pem(std::str::from_utf8(&pem)?)?,
            target,
            limits,
        })
    }

    /// A strictly verified approval to publish over `release` from `pin`.
    pub fn approve(
        &self,
        release: &GitObjectId,
        pin: &DoltReadPin,
        variant: u32,
    ) -> Result<DoltWriteApproval> {
        let release: ReleaseId = release.as_str().parse()?;
        let retained: ReleaseId = format!("{variant:064x}").parse()?;
        let header = self.selection.header();
        let operation = Operation::prepare_selection(
            release.clone(),
            retained,
            header.policy().clone(),
            self.selection.target_use().clone(),
            self.selection.semantic().clone(),
            header.toolchain().clone(),
            self.limits,
        )?;
        let base = DoltBase::new(release, pin.clone());
        self.sign(operation, Some(base), DoltWriteApproval::admit)
    }

    /// A strictly verified bootstrap approval: neither base nor mapping.
    pub fn bootstrap(&self, variant: u32) -> Result<DoltWriteApproval> {
        let header = self.bootstrap.header();
        let operation = Operation::prepare_admission(
            None,
            header.policy().clone(),
            vec![format!("{variant:064x}").parse()?],
            self.bootstrap.target_use().clone(),
            self.bootstrap.semantic().clone(),
            header.toolchain().clone(),
            self.limits,
        )?;
        self.sign(operation, None, DoltWriteApproval::bootstrap)
    }

    fn sign(
        &self,
        operation: Operation,
        base: Option<DoltBase>,
        admit: Admit,
    ) -> Result<DoltWriteApproval> {
        let operation =
            DoltOperation::prepare(operation, base, self.target.clone(), None, self.limits)?;
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
        Ok(admit(&supplied)?)
    }
}
