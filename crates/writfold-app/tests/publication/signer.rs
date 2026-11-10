//! Real signed Dolt approvals for disposable publication realms. The public
//! synthetic selection operation, policy and PUBLIC-TEST-ONLY key from
//! `writfold-wire/fixtures` are re-prepared with the actual predecessor
//! release and base pin, signed and strictly verified through the production
//! functions. `variant` changes the operation's retained-release field, so
//! each publication attempt is its own signed operation.
use super::world::Result;
use std::path::{Path, PathBuf};
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{GitObjectId, ReleaseId};
use writfold_store::backend::dolt::DoltWriteApproval;
use writfold_wire::Limits;
use writfold_wire::dolt_operation::{DoltBase, DoltOperation, DoltTarget};
use writfold_wire::operation::{Operation, OperationBody, PortfolioReleaseSelectionPlan};
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::signatures::dolt::verify_dolt_current;

pub struct Signer {
    selection: PortfolioReleaseSelectionPlan,
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
    pub fn load(target: DoltTarget) -> Result<Self> {
        let limits = Limits::new(1_000_000, 128, 10_000, 100_000)?;
        let vector = DoltOperation::decode(
            &fixture("dolt-operation", "selection.operation.json")?,
            limits,
        )?;
        let OperationBody::PortfolioSelection(selection) = vector.operation().body() else {
            return Err("the selection vector is not a portfolio selection".into());
        };
        let pem = fixture("signatures", "PUBLIC-TEST-ONLY.pem")?;
        Ok(Self {
            selection: selection.clone(),
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
        let operation = DoltOperation::prepare(
            operation,
            Some(DoltBase::new(release, pin.clone())),
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
}
