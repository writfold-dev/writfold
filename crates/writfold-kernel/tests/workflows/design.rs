use super::TestResult;
use writfold_domain::*;
use writfold_kernel::{design_workflow as design, workflow};

fn authored(status: AuthoredStatus) -> Result<AuthoredProposal, IdentityError> {
    let gate = AuthoredGateDefinition::new(
        "gate".parse()?,
        "works".into(),
        "exercise".into(),
        GateWhen::Land,
        Some("alice".parse()?),
        Some("checker".parse()?),
    );
    let result = AuthoredGateResult::new(
        gate.clone(),
        Some("old-target".parse()?),
        Some("checker".parse()?),
        GateOutcome::Pass,
        1.into(),
        20.into(),
        Some("design-v3:one".parse()?),
        "test".into(),
        "project".into(),
        vec!["evidence".into()],
        Some("alice".parse()?),
    );
    Ok(AuthoredProposal::new(
        status,
        AuthoredProfile::LegacyLimited,
        false,
        0.into(),
        Some("old-target".parse()?),
        vec![gate],
        vec![result],
        Some("alice".parse()?),
        Some("alice".parse()?),
        Some("bob".parse()?),
        None,
        vec!["landed".into()],
        "".into(),
        "p1".parse()?,
        None,
        vec![],
        Some("design-v3:one".parse()?),
        Some("design-v3:one".parse()?),
        "test".into(),
        "project".into(),
        vec![AuthoredLanded::new("landed".into(), "old-target".parse()?)],
    ))
}

#[test]
fn rebind_clears_landing_and_does_not_reuse_old_target_gates() -> TestResult {
    let current = DesignProposal::new(authored(AuthoredStatus::Accepted)?, "source".parse()?, None);
    assert!(design::design_candidate_valid(
        &current,
        AuthoredStatus::Accepted,
        &10.into()
    ));
    let binding = ImplementationBinding::new(
        "design-v3:one".parse()?,
        "source".parse()?,
        "new-target".parse()?,
        "alice".parse()?,
        10.into(),
    );
    let command = DesignBindingCommand::new(current.clone(), binding);
    let rebound =
        design::bind_implementation(&current, &command, &10.into()).ok_or("expected bind")?;
    assert!(rebound.authored().landed_refs().is_empty());
    assert!(rebound.authored().resolved_landed().is_empty());
    assert_eq!(
        rebound.authored().gate_results(),
        current.authored().gate_results()
    );
    assert_eq!(
        rebound.authored().accepted_revision(),
        current.authored().accepted_revision()
    );
    let gate = current
        .authored()
        .gate_definitions()
        .first()
        .ok_or("expected gate")?;
    assert!(workflow::authored_gate_passes(
        current.authored(),
        gate,
        &10.into()
    ));
    assert!(!workflow::authored_gate_passes(
        rebound.authored(),
        gate,
        &10.into()
    ));
    assert!(!design::design_candidate_valid(
        &rebound,
        AuthoredStatus::Applied,
        &10.into()
    ));
    assert!(design::bind_implementation(&rebound, &command, &10.into()).is_none());
    let same_target = ImplementationBinding::new(
        "design-v3:one".parse()?,
        "source".parse()?,
        "old-target".parse()?,
        "alice".parse()?,
        10.into(),
    );
    let same = design::bind_implementation(
        &current,
        &DesignBindingCommand::new(current.clone(), same_target),
        &10.into(),
    )
    .ok_or("same target bind")?;
    assert!(same.authored().landed_refs().is_empty());
    Ok(())
}

#[test]
fn reopen_clears_authority_but_finalized_records_refuse_reopen() -> TestResult {
    let current = DesignProposal::new(authored(AuthoredStatus::Accepted)?, "source".parse()?, None);
    let command =
        DesignReopenCommand::new(current.clone(), "carol".parse()?, "revise material".into());
    let reopened = design::reopen_design(&current, &command).ok_or("expected reopen")?;
    assert_eq!(*reopened.authored().status(), AuthoredStatus::Proposed);
    assert!(reopened.authored().accepted_revision().is_none());
    assert!(reopened.authored().acceptor().is_none());
    assert!(reopened.authored().executor().is_none());
    assert!(reopened.authored().verifier().is_none());
    assert!(reopened.authored().target_revision().is_none());
    assert!(reopened.binding().is_none());
    assert_eq!(
        reopened.authored().gate_results(),
        current.authored().gate_results()
    );
    assert!(design::reopen_design(&reopened, &command).is_none());
    for status in [AuthoredStatus::Applied, AuthoredStatus::Verified] {
        let finalized = DesignProposal::new(authored(status)?, "source".parse()?, None);
        let command =
            DesignReopenCommand::new(finalized.clone(), "carol".parse()?, "revise".into());
        assert!(design::reopen_design(&finalized, &command).is_none());
    }
    Ok(())
}

#[test]
fn authored_problems_keep_complete_reference_order() -> TestResult {
    let proposal = AuthoredProposal::new(
        AuthoredStatus::Landing,
        AuthoredProfile::Modern,
        false,
        0.into(),
        None,
        vec![],
        vec![],
        None,
        None,
        None,
        None,
        vec![],
        "".into(),
        "p1".parse()?,
        None,
        vec![],
        None,
        None,
        "".into(),
        "".into(),
        vec![],
    );
    assert_eq!(
        workflow::authored_candidate_problems(&proposal, AuthoredStatus::Applied, &10.into()),
        [
            "accepted proposal revision",
            "recorded actors",
            "review readiness",
            "acceptor",
            "executor",
            "explicit landed references"
        ]
    );
    Ok(())
}
