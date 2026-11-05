use super::fixture::{Fixture, TestResult, allowed, seconds};
use writfold_domain::{citation::*, *};
use writfold_kernel::citation::*;

#[test]
fn alias_lifetime_is_inclusive_and_access_is_rechecked_for_exact_material() -> TestResult {
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let handle: AliasId = "a".parse()?;
    let unknown: AliasId = "missing".parse()?;
    let mut table = AliasTable::new(f.scope(0));
    assert_eq!(
        table.issue(
            &handle,
            binding,
            seconds(10)?,
            seconds(20)?,
            allowed(binding)
        ),
        Ok(())
    );
    for now in [10, 15, 20] {
        assert_eq!(
            table.expand(f.scope(0), &handle, seconds(now)?, allowed(binding)),
            AliasResult::Durable(binding)
        );
    }
    for now in [9, 21] {
        assert_eq!(
            table.expand(f.scope(0), &handle, seconds(now)?, allowed(binding)),
            AliasResult::Expired
        );
    }
    assert_eq!(
        table.expand(f.scope(0), &unknown, seconds(15)?, allowed(binding)),
        AliasResult::Unavailable
    );
    assert_eq!(
        table.expand(
            f.scope(0),
            &handle,
            seconds(15)?,
            AccessDecision::new(binding, false)
        ),
        AliasResult::Denied
    );
    let role: EvidenceRoleId = "contrary".parse()?;
    let different = Binding::new(
        binding.id(),
        binding.release(),
        binding.context(),
        &role,
        binding.representation(),
        binding.selector(),
        binding.expected(),
    );
    assert_eq!(
        table.expand(f.scope(0), &handle, seconds(15)?, allowed(different)),
        AliasResult::Denied
    );
    assert_eq!(
        table
            .lookup(&handle)
            .ok_or("missing retained alias")?
            .binding()
            .role(),
        binding.role()
    );
    table.retire();
    assert_eq!(
        table.expand(f.scope(0), &handle, seconds(15)?, allowed(binding)),
        AliasResult::Expired
    );
    assert_eq!(
        table.issue(
            &unknown,
            binding,
            seconds(10)?,
            seconds(20)?,
            allowed(binding)
        ),
        Err(AliasIssueError::Expired)
    );
    Ok(())
}

#[test]
fn foreign_audience_and_stale_generation_precede_handle_existence() -> TestResult {
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let handle: AliasId = "missing".parse()?;
    let table = AliasTable::new(f.scope(1));
    let realm: RealmId = "other".parse()?;
    let principal: ActorId = "other".parse()?;
    let instance: InstanceId = "other".parse()?;
    let incarnation: SessionIncarnation = "other".parse()?;
    let release: SnapshotId = "other".parse()?;
    let context: ContextId = "other".parse()?;
    let g = CitationTableGeneration::from_u64(1);
    for dimension in 0..6 {
        let scope = AliasScope::new(
            if dimension == 0 {
                &realm
            } else {
                f.reference.realm()
            },
            if dimension == 1 {
                &principal
            } else {
                &f.principal
            },
            if dimension == 2 {
                &instance
            } else {
                &f.instance
            },
            if dimension == 3 {
                &incarnation
            } else {
                &f.incarnation
            },
            if dimension == 4 { &release } else { &f.release },
            if dimension == 5 { &context } else { &f.context },
            g,
        );
        assert_eq!(
            table.expand(scope, &handle, seconds(15)?, allowed(binding)),
            AliasResult::Foreign
        );
    }
    assert_eq!(
        table.expand(f.scope(0), &handle, seconds(15)?, allowed(binding)),
        AliasResult::Expired
    );
    assert_eq!(
        table.expand(f.scope(2), &handle, seconds(15)?, allowed(binding)),
        AliasResult::Expired
    );
    Ok(())
}

#[test]
fn issuance_refuses_foreign_scope_denial_and_invalid_lifetime_without_mutation() -> TestResult {
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let handle: AliasId = "a".parse()?;
    let mut table = AliasTable::new(f.scope(0));
    assert_eq!(
        table.issue(
            &handle,
            binding,
            seconds(20)?,
            seconds(10)?,
            allowed(binding)
        ),
        Err(AliasIssueError::InvalidLifetime)
    );
    assert_eq!(
        table.issue(
            &handle,
            binding,
            seconds(10)?,
            seconds(20)?,
            AccessDecision::new(binding, false)
        ),
        Err(AliasIssueError::Denied)
    );
    let release: SnapshotId = "other".parse()?;
    let foreign = Binding::new(
        binding.id(),
        &release,
        binding.context(),
        binding.role(),
        binding.representation(),
        binding.selector(),
        binding.expected(),
    );
    assert_eq!(
        table.issue(
            &handle,
            foreign,
            seconds(10)?,
            seconds(20)?,
            allowed(foreign)
        ),
        Err(AliasIssueError::Foreign)
    );
    assert!(table.entries().is_empty());
    Ok(())
}

#[test]
fn issuance_never_overwrites_an_existing_alias_even_with_equal_material() -> TestResult {
    let f = Fixture::new()?;
    let binding = f.binding(0, 1, b"a")?;
    let replacement = f.binding(1, 2, b"b")?;
    let handle: AliasId = "a".parse()?;
    let other: AliasId = "b".parse()?;
    let mut table = AliasTable::new(f.scope(0));
    assert_eq!(
        table.issue(
            &handle,
            binding,
            seconds(10)?,
            seconds(20)?,
            allowed(binding)
        ),
        Ok(())
    );
    for candidate in [binding, replacement] {
        assert_eq!(
            table.issue(
                &handle,
                candidate,
                seconds(30)?,
                seconds(40)?,
                allowed(candidate)
            ),
            Err(AliasIssueError::AlreadyIssued)
        );
    }
    assert_eq!(
        table.issue(
            &other,
            replacement,
            seconds(10)?,
            seconds(20)?,
            allowed(replacement)
        ),
        Ok(())
    );
    assert_eq!(
        table
            .lookup(&handle)
            .ok_or("missing original alias")?
            .binding(),
        binding
    );
    assert_eq!(
        table
            .lookup(&other)
            .ok_or("missing second alias")?
            .binding(),
        replacement
    );
    assert_eq!(table.entries().len(), 2);
    Ok(())
}
