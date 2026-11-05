use vstd::prelude::*;
use writfold_domain::{ByteOffset, citation::*};

verus! {
#[derive(Debug, Eq, PartialEq)]
pub struct ScanResult { candidates: Vec<Selector>, coverage: Coverage }
impl ScanResult {
    pub closed spec fn candidates_value(&self) -> Seq<Selector> { self.candidates@ }
    pub closed spec fn coverage_value(&self) -> Coverage { self.coverage }
    pub fn candidates(&self) -> (value: &[Selector])
        ensures value@ == self.candidates_value(),
    { self.candidates.as_slice() }
    pub fn coverage(&self) -> (value: Coverage)
        ensures value == self.coverage_value(),
    { self.coverage }
    pub fn into_parts(self) -> (value: (Vec<Selector>, Coverage))
        ensures value.0@ == self.candidates_value(), value.1 == self.coverage_value(),
    { (self.candidates, self.coverage) }
}

fn position_selector(offset: usize, width: usize) -> (selector: Option<Selector>)
    requires offset as int + width as int <= usize::MAX,
    ensures selector.is_some(),
        selector.unwrap()@ == (offset as nat, (offset + width) as nat, CoordinateUnit::Byte),
{
    let Ok(first) = ByteOffset::new(offset as u64) else { return None; };
    let Ok(last) = ByteOffset::new((offset + width) as u64) else { return None; };
    Selector::new(first, last, CoordinateUnit::Byte).ok()
}

/// Streams offsets, including the final empty occurrence; it never allocates all
/// possible positions. Reaching either caller limit stops before examining more
/// positions. A candidate cap therefore makes coverage partial, not silently complete.
/// `total` uses u128 so even the mathematical len + 1 empty-needle count cannot wrap.
pub fn diagnose(bytes: &[u8], needle: &[u8], budget: SearchBudget) -> (result: ScanResult)
    ensures
        result.candidates_value().len() <= budget.candidate_limit(),
        match result.coverage_value() {
            Coverage::Measured { examined, total } => examined <= budget.position_limit()
                && examined as u128 <= total
                && total == if needle.len() <= bytes.len() {
                    bytes.len() - needle.len() + 1
                } else { 0 }
                && (examined as u128 == total || examined == budget.position_limit()
                    || result.candidates_value().len() == budget.candidate_limit())
                && (forall|i: int| 0 <= i < result.candidates_value().len() ==>
                    result.candidates_value()[i]@.2 == CoordinateUnit::Byte
                    && result.candidates_value()[i]@.0 < examined
                    && result.candidates_value()[i]@.1 == result.candidates_value()[i]@.0 + needle.len()
                    && result.candidates_value()[i]@.1 <= bytes.len()
                    && bytes@.subrange(result.candidates_value()[i]@.0 as int,
                        result.candidates_value()[i]@.1 as int) == needle@)
                && (forall|position: int| 0 <= position < examined
                    && #[trigger] bytes@.subrange(position, position + needle.len()) == needle@
                    ==> exists|i: int| 0 <= i < result.candidates_value().len()
                        && result.candidates_value()[i]@.0 == position)
                && (forall|i: int, j: int| 0 <= i < j < result.candidates_value().len()
                    ==> result.candidates_value()[i]@.0 < result.candidates_value()[j]@.0),
            Coverage::Unavailable => false,
        },
{
    let total = if needle.len() <= bytes.len() {
        (bytes.len() - needle.len()) as u128 + 1
    } else { 0u128 };
    let mut candidates: Vec<Selector> = Vec::new();
    let mut examined = 0usize;
    while (examined as u128) < total && examined < budget.positions()
        && candidates.len() < budget.candidates()
        invariant examined <= budget.position_limit(), examined as u128 <= total,
            candidates.len() <= budget.candidate_limit(),
            total == if needle.len() <= bytes.len() { bytes.len() - needle.len() + 1 } else { 0 },
            forall|i: int| 0 <= i < candidates.len() ==>
                candidates@[i]@.2 == CoordinateUnit::Byte
                && candidates@[i]@.0 < examined && candidates@[i]@.1 <= bytes.len()
                && candidates@[i]@.1 == candidates@[i]@.0 + needle.len()
                && bytes@.subrange(candidates@[i]@.0 as int, candidates@[i]@.1 as int) == needle@,
            forall|position: int| 0 <= position < examined
                && #[trigger] bytes@.subrange(position, position + needle.len()) == needle@
                ==> exists|i: int| 0 <= i < candidates.len() && candidates@[i]@.0 == position,
            forall|i: int, j: int| 0 <= i < j < candidates.len()
                ==> candidates@[i]@.0 < candidates@[j]@.0,
        decreases budget.position_limit() - examined,
    {
        let ghost previous = candidates@;
        let last = examined + needle.len();
        if bytes_equal(&bytes[examined..last], needle) {
            // The helper proves Some for these bounds; the scan's completeness
            // postcondition independently forbids omitting a matching position.
            if let Some(selector) = position_selector(examined, needle.len()) {
                candidates.push(selector);
            }
        }
        proof {
            assert forall|position: int| 0 <= position <= examined
                && #[trigger] bytes@.subrange(position, position + needle.len()) == needle@
                implies exists|i: int| 0 <= i < candidates.len()
                    && candidates@[i]@.0 == position by {
                if position < examined {
                    let i = choose|i: int| 0 <= i < previous.len() && previous[i]@.0 == position;
                    assert(candidates@[i] == previous[i]);
                } else {
                    assert(candidates@[candidates.len() - 1]@.0 == position);
                }
            }
        }
        examined += 1;
    }
    ScanResult { candidates, coverage: Coverage::Measured { examined, total } }
}
}
