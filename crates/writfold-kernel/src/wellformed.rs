//! Whole-candidate invariants, independent of support and admission policy.
//! Nonempty is byte-nonempty, not whitespace normalization. Context snapshots
//! are checked by context_matches; context_wf deliberately does not bind a KB.
use vstd::prelude::*;
use writfold_domain::{Integer, *};
mod collections;
mod context;
pub mod graph;
mod rows;
mod membership;
#[cfg(verus_keep_ghost)]
mod lookups;
mod order;
mod sort;
mod index;
#[cfg(verus_keep_ghost)]
pub mod spec;
pub use context::context_wf;
pub use rows::{dependency_wf, well_formed};
pub(crate) use collections::unique;
#[cfg(verus_keep_ghost)]
use spec::*;

verus! {
pub fn nonempty(value: &str) -> (valid: bool)
    ensures valid == nonempty_spec(value@),
{
    !value.is_empty()
}
pub fn natural(value: &Integer) -> (valid: bool)
    ensures valid == natural_spec(value@),
{
    !value.is_negative()
}
pub fn ttl_wf(ttl: &Ttl) -> (valid: bool)
    ensures valid == ttl_wf_spec(ttl@),
{
    match ttl { Ttl::ExpireAfter(secs) => natural(secs), _ => true }
}
pub fn scope_wf(scope: &Scope) -> (valid: bool)
    ensures valid == scope_wf_spec(scope@),
{
    nonempty(scope.project().as_str())
}
fn has_enumeration(kb: &KB, id: &EnumerationId) -> (found: bool)
    ensures found == enumeration_present(kb@, id@),
{
    let values = kb.enumerations();
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(), values.deep_view() == kb@.enumerations,
            forall|j: int| 0 <= j < i ==> kb@.enumerations[j].id != id@,
        decreases values.len() - i,
    {
        let value = &values[i];
        proof { assert(value@ == kb@.enumerations[i as int]); }
        if value.id() == id {
            proof { assert(kb@.enumerations[i as int].id == id@); }
            return true;
        }
        i += 1;
    }
    false
}
fn has_attestation(kb: &KB, id: &AttestationId) -> (found: bool)
    ensures found == attestation_present(kb@, id@),
{
    let values = kb.attestations();
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(), values.deep_view() == kb@.attestations,
            forall|j: int| 0 <= j < i ==> kb@.attestations[j].id != id@,
        decreases values.len() - i,
    {
        let value = &values[i];
        proof { assert(value@ == kb@.attestations[i as int]); }
        if value.id() == id {
            proof { assert(kb@.attestations[i as int].id == id@); }
            return true;
        }
        i += 1;
    }
    false
}
pub fn formula_wf(formula: &Formula, kb: &KB) -> (valid: bool)
    ensures valid == formula_wf_spec(formula@, kb@),
    decreases formula,
{
    match formula {
        Formula::Atomic(atom) => nonempty(atom.subject()) && nonempty(atom.predicate()),
        Formula::Truth => true,
        Formula::Conjunction { left, right } => formula_wf(left, kb) && formula_wf(right, kb),
        Formula::All { enumeration, predicate } => nonempty(predicate) && has_enumeration(kb, enumeration),
        Formula::Absent { enumeration } | Formula::Inventory { enumeration } => has_enumeration(kb, enumeration),
    }
}
pub fn statement_wf(statement: &Statement, kb: &KB) -> (valid: bool)
    ensures valid == statement_wf_spec(statement@, kb@),
{
    scope_wf(statement.scope()) && formula_wf(statement.formula(), kb)
        && (*statement.polarity() == Polarity::Positive || matches!(statement.formula(), Formula::Atomic(_)))
}
pub fn witness_wf(witness: &Witness, kb: &KB) -> (valid: bool)
    ensures valid == witness_wf_spec(witness@, kb@),
    decreases witness,
{
    match witness {
        Witness::Attest { attestation } => has_attestation(kb, attestation),
        Witness::Truth { scope } => scope_wf(scope),
        Witness::And { left, right } => witness_wf(left, kb) && witness_wf(right, kb),
        Witness::Specialize { scope, premise } => scope_wf(scope) && witness_wf(premise, kb),
        Witness::All { enumeration, predicate, premise } => has_enumeration(kb, enumeration)
            && nonempty(predicate) && witness_wf(premise, kb),
        Witness::Absent { enumeration, premise } => has_enumeration(kb, enumeration) && witness_wf(premise, kb),
    }
}
}
