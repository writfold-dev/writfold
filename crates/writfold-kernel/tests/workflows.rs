use std::{borrow::Cow, error::Error};
use writfold_domain::*;
use writfold_kernel::portfolio;
type TestResult = Result<(), Box<dyn Error>>;

#[path = "workflows/design.rs"]
mod design;

fn context(snapshot: &str, sources: Vec<SourceSelection>) -> Result<Context, IdentityError> {
    Ok(Context::new(
        snapshot.parse()?,
        "2.0.0".into(),
        "core-2".parse()?,
        "kctx-2".parse()?,
        Policy::new(
            "policy".parse()?,
            false,
            false,
            true,
            Ttl::NoExpiry,
            vec!["alice".parse()?],
        ),
        10.into(),
        Scope::new("project".parse()?, "".into()),
        "review".into(),
        sources,
    ))
}

fn truth_kb(snapshot: &str) -> Result<KB, IdentityError> {
    let scope = Scope::new("project".parse()?, "".into());
    let claim = Claim::new(
        "claim".parse()?,
        "r1".parse()?,
        Statement::new(Formula::Truth, scope.clone(), Polarity::Positive),
    );
    let derivation = Derivation::new(
        "derive".parse()?,
        claim.claim_ref(),
        Witness::Truth { scope },
    );
    Ok(KB::new(
        snapshot.parse()?,
        vec![],
        vec![claim],
        vec![],
        vec![],
        vec![derivation],
        vec![],
        vec![],
    ))
}

fn truth_plan(state: &PortfolioState) -> Result<PortfolioPlan, IdentityError> {
    let candidate = truth_kb("candidate")?;
    let assignment = ClaimAssignment::new(
        ClaimRef::new("claim".parse()?, "r1".parse()?),
        "main".parse()?,
        Witness::Truth {
            scope: Scope::new("project".parse()?, "".into()),
        },
    );
    Ok(PortfolioPlan::new(
        "operation".parse()?,
        state.kb().snapshot().clone(),
        candidate,
        "alice".parse()?,
        vec![NamedContext::new(
            "main".parse()?,
            context("candidate", vec![])?,
        )],
        vec![assignment],
        vec![],
    ))
}

#[test]
fn complete_plan_replay_precedes_predecessor_and_authority() -> TestResult {
    let state = PortfolioState::new(KB::empty("base".parse()?), vec![], vec![]);
    let plan = truth_plan(&state)?;
    let approval = ModelPortfolioApproval::new(plan.clone());
    let result = portfolio::apply_portfolio(&state, &[], &plan, &[approval]);
    assert_eq!(*result.outcome(), CommitOutcome::Applied);
    let replay = portfolio::apply_portfolio(result.state(), result.journal(), &plan, &[]);
    assert_eq!(*replay.outcome(), CommitOutcome::AlreadyApplied);
    let changed = PortfolioPlan::new(
        plan.id().clone(),
        plan.expected_snapshot().clone(),
        plan.candidate().clone(),
        "mallory".parse()?,
        plan.contexts().clone(),
        plan.assignments().clone(),
        plan.retired_contexts().clone(),
    );
    let conflict = portfolio::apply_portfolio(result.state(), result.journal(), &changed, &[]);
    assert_eq!(*conflict.outcome(), CommitOutcome::InputConflict);
    assert!(std::ptr::eq(conflict.state(), result.state()));
    assert_eq!(conflict.journal(), result.journal());
    let (state_result, journal_result, _) = conflict.into_parts();
    assert!(matches!(state_result, Cow::Borrowed(_)));
    assert!(matches!(journal_result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn admission_requires_exact_assignments_and_explicit_retirements() -> TestResult {
    let state = PortfolioState::new(KB::empty("base".parse()?), vec![], vec![]);
    let plan = truth_plan(&state)?;
    assert!(portfolio::portfolio_plan_valid(&state, &plan));
    let omitted = PortfolioPlan::new(
        plan.id().clone(),
        plan.expected_snapshot().clone(),
        plan.candidate().clone(),
        plan.actor().clone(),
        plan.contexts().clone(),
        vec![],
        vec![],
    );
    assert!(!portfolio::portfolio_plan_valid(&state, &omitted));
    let rejected = portfolio::apply_portfolio(
        &state,
        &[],
        &omitted,
        &[ModelPortfolioApproval::new(omitted.clone())],
    );
    assert_eq!(*rejected.outcome(), CommitOutcome::Rejected);
    assert!(std::ptr::eq(rejected.state(), &state));
    assert!(rejected.journal().is_empty());
    let prior = PortfolioState::new(
        state.kb().clone(),
        vec![],
        vec![NamedContext::new("old".parse()?, context("base", vec![])?)],
    );
    assert!(!portfolio::portfolio_plan_valid(&prior, &plan));
    let explicit = PortfolioPlan::new(
        plan.id().clone(),
        plan.expected_snapshot().clone(),
        plan.candidate().clone(),
        plan.actor().clone(),
        plan.contexts().clone(),
        plan.assignments().clone(),
        vec!["old".parse()?],
    );
    assert!(portfolio::portfolio_plan_valid(&prior, &explicit));
    Ok(())
}

#[test]
fn assignments_cannot_borrow_another_contexts_sources() -> TestResult {
    let scope = Scope::new("project".parse()?, "".into());
    let statement = Statement::new(
        Formula::Atomic(Atom::new("asset".into(), "works".into(), vec![])),
        scope,
        Polarity::Positive,
    );
    let claim = Claim::new("claim".parse()?, "r1".parse()?, statement.clone());
    let witness = Witness::Attest {
        attestation: "attestation".parse()?,
    };
    let observation = Observation::new(
        "observation".parse()?,
        "source".parse()?,
        "rev".parse()?,
        "bytes".parse()?,
        1.into(),
        ObsOutcome::Ok,
    );
    let attestation = Attestation::new(
        "attestation".parse()?,
        claim.claim_ref(),
        observation.id().clone(),
        statement,
        "alice".parse()?,
    );
    let kb = KB::new(
        "candidate".parse()?,
        vec![observation],
        vec![claim.clone()],
        vec![attestation],
        vec![],
        vec![Derivation::new(
            "derivation".parse()?,
            claim.claim_ref(),
            witness.clone(),
        )],
        vec![],
        vec![],
    );
    let contexts = vec![
        NamedContext::new("empty".parse()?, context("candidate", vec![])?),
        NamedContext::new(
            "source".parse()?,
            context(
                "candidate",
                vec![SourceSelection::new(
                    "source".parse()?,
                    "rev".parse()?,
                    "bytes".parse()?,
                )],
            )?,
        ),
    ];
    let correct = ClaimAssignment::new(claim.claim_ref(), "source".parse()?, witness.clone());
    let borrowed = ClaimAssignment::new(claim.claim_ref(), "empty".parse()?, witness);
    assert!(portfolio::assignment_valid(&kb, &contexts, &correct));
    assert!(!portfolio::assignment_valid(&kb, &contexts, &borrowed));
    let state = PortfolioState::new(kb, vec![claim.claim_ref()], contexts);
    assert!(!portfolio::query_context_allowed(
        &state,
        &"empty".parse()?,
        &context(
            "candidate",
            vec![SourceSelection::new(
                "source".parse()?,
                "rev".parse()?,
                "bytes".parse()?
            )]
        )?
    ));
    Ok(())
}

fn retracted_state(
    old_kb: &KB,
    reference: &ClaimRef,
) -> Result<(PortfolioState, Correction), IdentityError> {
    let correction = Correction::new(
        "retraction".parse()?,
        CorrectionKind::Retract,
        reference.clone(),
        None,
        5.into(),
    );
    let current_kb = KB::new(
        "current".parse()?,
        vec![],
        old_kb.claims().clone(),
        vec![],
        vec![],
        old_kb.derivations().clone(),
        vec![correction.clone()],
        vec![],
    );
    let state = PortfolioState::new(
        current_kb,
        vec![],
        vec![NamedContext::new(
            "main".parse()?,
            context("current", vec![])?,
        )],
    );
    Ok((state, correction))
}

#[test]
fn forward_selection_keeps_current_corrections_and_context_policy() -> TestResult {
    let old_kb = truth_kb("old")?;
    let reference = ClaimRef::new("claim".parse()?, "r1".parse()?);
    let old = PortfolioState::new(
        old_kb.clone(),
        vec![reference.clone()],
        vec![NamedContext::new("main".parse()?, context("old", vec![])?)],
    );
    let (state, correction) = retracted_state(&old_kb, &reference)?;
    let plan = PortfolioSelectionPlan::new(
        "select".parse()?,
        "current".parse()?,
        "old".parse()?,
        "next".parse()?,
        "alice".parse()?,
        vec![NamedContext::new("main".parse()?, context("next", vec![])?)],
        vec![reference.clone()],
    );
    let result = portfolio::apply_portfolio_selection(
        &state,
        &[],
        &plan,
        &[ModelPortfolioSelectionApproval::new(plan.clone())],
        &[old],
    );
    assert_eq!(*result.outcome(), CommitOutcome::Applied);
    assert_eq!(result.state().kb().corrections(), &[correction]);
    assert_eq!(result.state().selected(), std::slice::from_ref(&reference));
    let assessment = portfolio::assess_portfolio(
        result.state(),
        &"main".parse()?,
        &context("next", vec![])?,
        &reference,
    );
    assert!(assessment.is_some_and(|assessment| !assessment.usable()));
    let old_policy = Policy::new(
        "old-policy".parse()?,
        false,
        false,
        false,
        Ttl::NoExpiry,
        vec!["alice".parse()?],
    );
    let context_now = context("next", vec![])?;
    let substituted = Context::new(
        context_now.snapshot().clone(),
        context_now.semantic_version().clone(),
        context_now.ruleset().clone(),
        context_now.checker().clone(),
        old_policy,
        context_now.now().clone(),
        context_now.target().clone(),
        context_now.intended_use().clone(),
        vec![],
    );
    assert!(!portfolio::selection_contexts_valid(
        state.contexts(),
        &[NamedContext::new("main".parse()?, substituted)],
        plan.new_snapshot()
    ));
    Ok(())
}
