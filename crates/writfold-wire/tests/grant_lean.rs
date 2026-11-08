//! Step-by-step agreement between Lean `environment` grant transitions,
//! `currentGrant` and the `intentReady` grant conjunct, and the Rust authority
//! kernel, on the exported scenarios. Every step's outcome, clock, installed
//! grant list (newest first, with revocation flags), current grant and coverage
//! must match. Finite executable agreement, not a refinement proof.
use serde_json::Value;
use std::collections::BTreeSet;
use writfold_domain::engineering::approval::{Signer, Validity};
use writfold_domain::engineering::execution::Capability;
use writfold_domain::engineering::grant::Grant;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_domain::worktree::Scope;
use writfold_domain::{EpochSeconds, ExecutionCost, PolicyDigest, ResourceId};
use writfold_kernel::authority::{Authority, Authorization};

type Fallible<T = ()> = Result<T, Box<dyn std::error::Error>>;
const CASES: &str = include_str!("fixtures/grant-lean-cases.json");

fn num(v: &Value, key: &str) -> Fallible<u64> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing number {key}").into())
}
fn field<'a>(v: &'a Value, key: &str) -> Fallible<&'a Value> {
    v.get(key).ok_or_else(|| format!("missing {key}").into())
}
fn nums(v: &Value, key: &str) -> Fallible<Vec<u64>> {
    field(v, key)?
        .as_array()
        .ok_or("array")?
        .iter()
        .map(|n| n.as_u64().ok_or_else(|| "number".into()))
        .collect()
}
fn blob(n: u64) -> BlobDigest {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&n.to_be_bytes());
    BlobDigest::from_digest(bytes)
}
fn policy(n: u64) -> Fallible<PolicyDigest> {
    Ok(format!("{n:064x}").parse()?)
}
fn seconds(n: u64) -> Fallible<EpochSeconds> {
    Ok(EpochSeconds::new(n)?)
}
fn scope(run: u64, realm: u64, subject: u64) -> Fallible<Scope> {
    Ok(Scope::new(
        format!("realm-{realm}").parse()?,
        format!("run-{run}").parse()?,
        "repo".parse()?,
        "wt".parse()?,
        format!("binding-{subject}").parse()?,
    )?)
}
fn who(v: &Value) -> Fallible<Scope> {
    scope(num(v, "run")?, num(v, "realm")?, num(v, "subject")?)
}
fn resources(v: &Value, key: &str) -> Fallible<Vec<ResourceId>> {
    nums(v, key)?
        .into_iter()
        .map(|n| Ok(format!("resource-{n}").parse()?))
        .collect()
}
fn capability(v: &Value) -> Fallible<Capability> {
    Ok(match v.as_str().ok_or("capability")? {
        "read" => Capability::Read,
        "check" => Capability::Check,
        "write" => Capability::Write,
        "publish" => Capability::Publish,
        other => return Err(format!("unknown capability {other}").into()),
    })
}
fn grant(g: &Value) -> Fallible<Grant> {
    let capabilities = field(g, "capabilities")?
        .as_array()
        .ok_or("capabilities")?
        .iter()
        .map(capability)
        .collect::<Fallible<Vec<_>>>()?;
    Ok(Grant::new(
        format!("grant-{}", num(g, "id")?).parse()?,
        scope(num(g, "run")?, num(g, "realm")?, num(g, "subject")?)?,
        resources(g, "resources")?,
        capabilities,
        ExecutionCost::new(num(g, "budget")?)?,
        Validity::new(
            seconds(num(g, "valid_from")?)?,
            seconds(num(g, "expires")?)?,
        )?,
        Signer::new("operator-key".parse()?, policy(1)?),
    )?)
}
/// Lean's grant list as (id, revoked), newest first.
fn lean_grants(step: &Value) -> Fallible<Vec<(String, bool)>> {
    field(step, "grants_after")?
        .as_array()
        .ok_or("grants_after")?
        .iter()
        .map(|g| {
            let revoked = field(g, "revoked")?.as_bool().ok_or("revoked")?;
            Ok((format!("grant-{}", num(g, "id")?), revoked))
        })
        .collect()
}
fn rust_grants(authority: &Authority) -> Vec<(String, bool)> {
    authority
        .grants()
        .iter()
        .map(|g| (g.grant().id().as_str().to_owned(), g.revoked()))
        .collect()
}

/// Apply one step; returns Rust's outcome. An already-revoked grant has no
/// Rust spelling (installation always starts unrevoked), so Lean must refuse it.
fn apply(
    authority: &mut Authority,
    retained: &BTreeSet<BlobDigest>,
    step: &Value,
) -> Fallible<bool> {
    let g = step.get("grant").unwrap_or(&Value::Null);
    Ok(match field(step, "kind")?.as_str().ok_or("kind")? {
        "install" if field(g, "revoked")?.as_bool() == Some(true) => false,
        "install" => authority
            .install(
                Authorization {
                    realm: format!("realm-{}", num(step, "realm")?).parse()?,
                    policy: policy(num(step, "policy")?)?,
                    grant: grant(g)?,
                    envelope: blob(num(step, "envelope")?),
                },
                retained,
            )
            .is_ok(),
        "revoke" => {
            authority.revoke(&format!("grant-{}", g.as_u64().ok_or("grant")?).parse()?);
            true
        }
        "advance" => authority.advance(seconds(num(step, "time")?)?).is_ok(),
        other => return Err(format!("unknown step {other}").into()),
    })
}

fn query(authority: &Authority, step: &Value) -> Fallible {
    let scope = who(field(step, "who")?)?;
    let current = authority
        .current(&scope)
        .map(|g| g.id().as_str().to_owned());
    let lean = field(step, "current_grant")?
        .as_u64()
        .map(|n| format!("grant-{n}"));
    assert_eq!(current, lean, "current grant");
    let covers = authority.covers(
        &scope,
        capability(field(step, "capability")?)?,
        &resources(step, "resources")?,
    );
    assert_eq!(Some(covers), field(step, "covers")?.as_bool(), "covers");
    Ok(())
}

#[test]
fn rust_authority_agrees_with_lean_on_every_step() -> Fallible {
    let doc: Value = serde_json::from_str(CASES)?;
    assert_eq!(doc["schema"], "writfold-grant-lean-cases/1");
    let initial = field(&doc, "initial")?;
    let retained: BTreeSet<_> = nums(initial, "retained")?.into_iter().map(blob).collect();
    let scenarios = field(&doc, "scenarios")?.as_array().ok_or("scenarios")?;
    let (mut steps, mut queries, mut refusals) = (0, 0, 0);
    for scenario in scenarios {
        let name = field(scenario, "name")?;
        let mut authority = Authority::new(
            format!("realm-{}", num(initial, "realm")?).parse()?,
            policy(num(initial, "policy")?)?,
            seconds(num(initial, "now")?)?,
        );
        for step in field(scenario, "steps")?.as_array().ok_or("steps")? {
            if step["kind"] == "query" {
                query(&authority, step).map_err(|e| format!("{name}: {e}"))?;
                queries += 1;
            } else {
                let ok = apply(&mut authority, &retained, step)?;
                assert_eq!(Some(ok), step["lean_ok"].as_bool(), "{name}: {step}");
                refusals += usize::from(!ok);
            }
            assert_eq!(
                authority.now().get(),
                num(step, "now_after")?,
                "{name}: clock"
            );
            assert_eq!(
                rust_grants(&authority),
                lean_grants(step)?,
                "{name}: grants"
            );
            steps += 1;
        }
    }
    assert_eq!(
        (scenarios.len(), steps, queries, refusals),
        (26, 79, 34, 11)
    );
    Ok(())
}
