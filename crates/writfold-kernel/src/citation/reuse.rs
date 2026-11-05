use super::{ResolveResult, resolve};
use vstd::prelude::*;
use writfold_domain::{ClaimRef, Context, ContextId, Dependency, KB, Receipt, citation::*};
#[cfg(verus_keep_ghost)]
use writfold_domain::{DependencyView, ReceiptView};

verus! {
/// An association with an existing receipt; construction neither validates it nor
/// changes its witness, dependencies, observation times or validity interval.
#[derive(Debug)]
pub struct BindingValidation<'a> {
    legacy: &'a Receipt, context_id: &'a ContextId, bindings: &'a [Binding<'a>],
}
impl<'a> BindingValidation<'a> {
    pub closed spec fn legacy_value(&self) -> ReceiptView { self.legacy@ }
    pub closed spec fn context_value(&self) -> Seq<char> { self.context_id@ }
    pub closed spec fn bindings_value(&self) -> Seq<Binding<'a>> { self.bindings@ }
    pub fn new(legacy: &'a Receipt, context_id: &'a ContextId, bindings: &'a [Binding<'a>])
        -> (value: Self)
        ensures value.legacy_value() == legacy@, value.context_value() == context_id@,
            value.bindings_value() == bindings@,
    { Self { legacy, context_id, bindings } }
    pub fn legacy(&self) -> (value: &'a Receipt)
        ensures value@ == self.legacy_value(),
    { self.legacy }
    pub fn context_id(&self) -> (value: &'a ContextId)
        ensures value@ == self.context_value(),
    { self.context_id }
    pub fn bindings(&self) -> (value: &'a [Binding<'a>])
        ensures value@ == self.bindings_value(),
    { self.bindings }
}

pub open spec fn observation_material(dependency: DependencyView, binding: BindingView) -> bool {
    match dependency {
        DependencyView::Observation(observation) => observation.source == binding.representation.repository
            && observation.revision == binding.representation.revision
            && observation.content == binding.representation.content,
        _ => false,
    }
}

pub fn binding_in_footprint(dependencies: &[Dependency], binding: Binding<'_>) -> (present: bool)
    ensures present == (exists|i: int| 0 <= i < dependencies.len()
        && observation_material(dependencies.deep_view()[i], binding@)),
{
    let mut index = 0;
    while index < dependencies.len()
        invariant index <= dependencies.len(),
            forall|i: int| 0 <= i < index ==> !observation_material(dependencies.deep_view()[i], binding@),
        decreases dependencies.len() - index,
    {
        let matches = match &dependencies[index] {
            Dependency::Observation(observation) =>
                observation.source() == binding.representation().repository()
                && observation.revision() == binding.representation().revision()
                && observation.content() == binding.representation().content(),
            _ => false,
        };
        if matches {
            proof {
                assert(observation_material(dependencies.deep_view()[index as int], binding@));
            }
            return true;
        }
        index += 1;
    }
    false
}

/// Adds exact citation association checks to the actual legacy reuse judgment.
/// No new receipt, witness, observation or time is produced. Access decisions must
/// correspond, in order, to the full current binding list and current policy.
#[allow(clippy::too_many_arguments, reason = "composition with unchanged legacy reuse inputs")]
pub fn citation_reusable(store: &[Representation<'_>], access: &[AccessDecision<'_>],
    kb: &KB, context: &Context, context_id: &ContextId, claim: &ClaimRef,
    current: &[Binding<'_>], validation: &BindingValidation<'_>) -> (reusable: bool)
    ensures reusable ==> crate::checker::spec::reusable_spec(kb@, context@, claim@, validation.legacy_value()),
        reusable ==> context_id@ == validation.context_value()
            && current.len() == validation.bindings_value().len() && access.len() == current.len(),
        reusable ==> forall|i: int| 0 <= i < current.len() ==>
            current@[i]@ == validation.bindings_value()[i]@
            && current@[i]@.context == context_id@
            && access@[i].permitted_value() && access@[i].binding_value() == current@[i]@
            && exists|j: int| 0 <= j < validation.legacy_value().dependencies.len()
                && match #[trigger] validation.legacy_value().dependencies[j] {
                    DependencyView::Observation(observation) =>
                        observation.source == current@[i]@.representation.repository
                        && observation.revision == current@[i]@.representation.revision
                        && observation.content == current@[i]@.representation.content,
                    _ => false,
                },
        reusable ==> forall|i: int| 0 <= i < current.len() ==>
            current@[i]@.representation.profile == RepresentationProfile::GitBlob
            && current@[i]@.selector.2 == CoordinateUnit::Byte
            && exists|j: int| 0 <= j < store.len()
                && store@[j].reference_value() == current@[i]@.representation
                && current@[i]@.selector.0 <= current@[i]@.selector.1 <= store@[j].bytes_value().len()
                && current@[i]@.expected == store@[j].bytes_value().subrange(
                    current@[i]@.selector.0 as int, current@[i]@.selector.1 as int),
{
    if context_id != validation.context_id || current.len() != validation.bindings.len()
        || access.len() != current.len() { return false; }
    let mut index = 0;
    while index < current.len()
        invariant index <= current.len(), current.len() == validation.bindings.len(),
            access.len() == current.len(), context_id@ == validation.context_value(),
            forall|i: int| 0 <= i < index ==> current@[i]@ == validation.bindings_value()[i]@
                && current@[i]@.context == context_id@
                && access@[i].permitted_value() && access@[i].binding_value() == current@[i]@
                && exists|j: int| 0 <= j < validation.legacy_value().dependencies.len()
                    && observation_material(validation.legacy_value().dependencies[j], current@[i]@),
            forall|i: int| 0 <= i < index ==>
                current@[i]@.representation.profile == RepresentationProfile::GitBlob
                && current@[i]@.selector.2 == CoordinateUnit::Byte
                && exists|j: int| 0 <= j < store.len()
                    && store@[j].reference_value() == current@[i]@.representation
                    && current@[i]@.selector.0 <= current@[i]@.selector.1 <= store@[j].bytes_value().len()
                    && current@[i]@.expected == store@[j].bytes_value().subrange(
                        current@[i]@.selector.0 as int, current@[i]@.selector.1 as int),
        decreases current.len() - index,
    {
        let binding = current[index];
        if !binding.same(validation.bindings[index]) || binding.context() != context_id
            || !binding_in_footprint(validation.legacy.dependencies(), binding) { return false; }
        if !matches!(resolve(store, binding, access[index]), ResolveResult::Exact { .. }) { return false; }
        index += 1;
    }
    crate::checker::reusable(kb, context, claim, validation.legacy)
}
}
