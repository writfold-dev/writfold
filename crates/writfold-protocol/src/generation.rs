//! Generation allocation within one runtime-owned instance/session namespace.
//! A caller must retain the high-water mark and all live generations until reaping.
use vstd::prelude::*;
use writfold_domain::RequestGeneration;

verus! {

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationAdvance {
    Allocated(RequestGeneration),
    InUse,
    Exhausted,
}

/// Check the entire retained generation set without copying it.
pub fn generation_is_live(active: &[RequestGeneration], wanted: RequestGeneration) -> (found: bool)
    ensures
        found <==> exists|i: int| 0 <= i < active.len()
            && active@[i].value() == wanted.value(),
{
    let mut index = 0usize;
    while index < active.len()
        invariant
            index <= active.len(),
            forall|i: int| 0 <= i < index ==> active@[i].value() != wanted.value(),
        decreases active.len() - index,
    {
        if active[index].get() == wanted.get() {
            return true;
        }
        index += 1;
    }
    false
}

/// Never wrap or reuse an occupied generation. Cancellation alone cannot remove
/// an entry from `active`; the runtime must first observe owned-child teardown.
pub fn advance_generation(previous: RequestGeneration, active: &[RequestGeneration]) -> (outcome: GenerationAdvance)
    ensures
        (outcome is Exhausted) <==> previous.value() == u64::MAX,
        (outcome is InUse) <==> (previous.value() < u64::MAX
            && exists|i: int| 0 <= i < active.len()
                && active@[i].value() == previous.value() + 1),
        match outcome {
            GenerationAdvance::Allocated(next) => next.value() == previous.value() + 1
                && !exists|i: int| 0 <= i < active.len()
                    && active@[i].value() == next.value(),
            _ => true,
        },
{
    let next = match previous.checked_next() {
        Some(next) => next,
        None => return GenerationAdvance::Exhausted,
    };
    if generation_is_live(active, next) {
        GenerationAdvance::InUse
    } else {
        GenerationAdvance::Allocated(next)
    }
}

}
