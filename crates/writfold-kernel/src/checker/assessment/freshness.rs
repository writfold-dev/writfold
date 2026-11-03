use super::super::context::Frame;
use super::super::footprint::Evaluation;
use super::super::input::Input;
#[cfg(verus_keep_ghost)]
use super::super::assessment_model::*;
#[cfg(verus_keep_ghost)]
use super::super::input::{InputView, dependency_value};
use crate::time::assess_fresh;
use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub fn witness_freshness(context: &Context, dependencies: &[Dependency]) -> (freshness: Freshness)
    ensures freshness == witness_freshness_spec(context@, dependencies.deep_view()),
{
    proof { assert(dependencies.deep_view().len() == dependencies.len()); }
    let mut index = 0;
    let mut any = false;
    let mut unknown = false;
    while index < dependencies.len()
        invariant index <= dependencies.len(), dependencies.deep_view().len() == dependencies.len(),
            any == (exists|i: int| 0 <= i < index && observation_state(context@, dependencies.deep_view()[i]).is_some()),
            unknown == (exists|i: int| 0 <= i < index && observation_state(context@, dependencies.deep_view()[i]) == Some(Freshness::Unknown)),
            forall|i: int| 0 <= i < index ==> observation_state(context@, dependencies.deep_view()[i]) != Some(Freshness::Stale),
        decreases dependencies.len() - index,
    {
        if let Dependency::Observation(observation) = &dependencies[index] {
            any = true;
            let state = assess_fresh(context.policy().ttl(), observation.observed_at(), context.now());
            if state == Freshness::Stale {
                proof { assert(observation_state(context@, dependencies.deep_view()[index as int]) == Some(Freshness::Stale)); }
                return Freshness::Stale;
            }
            if state == Freshness::Unknown { unknown = true; }
        }
        index += 1;
    }
    if any && !unknown { Freshness::Fresh } else { Freshness::Unknown }
}

fn input_freshness(context: &Context, dependencies: &[Input<'_>]) -> (freshness: Freshness)
    ensures freshness == input_freshness_spec(context@, dependencies.deep_view()),
{
    let ghost values = dependencies.deep_view().map(|i: int, input: InputView| dependency_value(input));
    proof { assert(values.len() == dependencies.len()); }
    let mut index = 0;
    let mut any = false;
    let mut unknown = false;
    while index < dependencies.len()
        invariant index <= dependencies.len(), values.len() == dependencies.len(),
            values == dependencies.deep_view().map(|i: int, input: InputView| dependency_value(input)),
            any == (exists|i: int| 0 <= i < index && observation_state(context@, values[i]).is_some()),
            unknown == (exists|i: int| 0 <= i < index && observation_state(context@, values[i]) == Some(Freshness::Unknown)),
            forall|i: int| 0 <= i < index ==> observation_state(context@, values[i]) != Some(Freshness::Stale),
        decreases dependencies.len() - index,
    {
        proof { assert(values[index as int] == dependency_value(dependencies.deep_view()[index as int])); }
        if let Input::Observation(observation) = &dependencies[index] {
            any = true;
            let state = assess_fresh(context.policy().ttl(), observation.observed_at(), context.now());
            if state == Freshness::Stale {
                proof { assert(observation_state(context@, values[index as int]) == Some(Freshness::Stale)); }
                return Freshness::Stale;
            }
            if state == Freshness::Unknown { unknown = true; }
        }
        index += 1;
    }
    if any && !unknown { Freshness::Fresh } else { Freshness::Unknown }
}

pub(super) fn alternative_freshness(context: Frame<'_>, accepted: &[(&Witness, Evaluation<'_>)]) -> (freshness: Freshness)
    ensures freshness == alternative_freshness_spec(context@.base, accepted.deep_view()),
{
    let mut all_stale = !accepted.is_empty();
    let mut index = 0;
    while index < accepted.len()
        invariant index <= accepted.len(),
            forall|i: int| 0 <= i < index ==> input_freshness_spec(context@.base, accepted.deep_view()[i].1.dependencies) != Freshness::Fresh,
            all_stale == (accepted.len() > 0 && (forall|i: int| 0 <= i < index
                ==> input_freshness_spec(context@.base, accepted.deep_view()[i].1.dependencies) == Freshness::Stale)),
        decreases accepted.len() - index,
    {
        let state = input_freshness(context.base, &accepted[index].1.dependencies);
        if state == Freshness::Fresh {
            proof { assert(input_freshness_spec(context@.base, accepted.deep_view()[index as int].1.dependencies) == Freshness::Fresh); }
            return Freshness::Fresh;
        }
        all_stale = all_stale && state == Freshness::Stale;
        index += 1;
    }
    if all_stale { Freshness::Stale } else { Freshness::Unknown }
}

} // verus!
