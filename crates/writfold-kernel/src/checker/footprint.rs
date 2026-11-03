use super::context::Frame;
use super::input::Input;
#[cfg(verus_keep_ghost)]
use super::input::{InputView, dependency_value};
use std::borrow::Cow;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::iter::IteratorSpec;
use writfold_domain::*;

verus! {


pub(super) fn into_formula(value: Cow<'_, Formula>) -> (formula: Formula)
    ensures formula@ == value@,
{
    match value {
        Cow::Borrowed(formula) => formula.clone(),
        Cow::Owned(formula) => formula,
    }
}

pub struct Evaluation<'a> {
    pub formula: Cow<'a, Formula>,
    pub scope: &'a Scope,
    pub polarity: Polarity,
    pub dependencies: Vec<Input<'a>>,
}

#[cfg(verus_keep_ghost)]
pub ghost struct EvaluationView {
    pub formula: FormulaView,
    pub scope: ScopeView,
    pub polarity: Polarity,
    pub dependencies: Seq<InputView>,
}
#[cfg(verus_keep_ghost)]
impl View for Evaluation<'_> {
    type V = EvaluationView;
    open spec fn view(&self) -> EvaluationView {
        EvaluationView {
            formula: self.formula@, scope: self.scope@, polarity: self.polarity,
            dependencies: self.dependencies.deep_view(),
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Evaluation<'_> {
    type V = EvaluationView;
    open spec fn deep_view(&self) -> EvaluationView { self@ }
}

pub open spec fn statement_value(value: EvaluationView) -> StatementView {
    StatementView { formula: value.formula, scope: value.scope, polarity: value.polarity }
}
pub open spec fn checked_value(value: EvaluationView) -> CheckedView {
    CheckedView { statement: statement_value(value),
        dependencies: value.dependencies.map_values(|input: InputView| dependency_value(input)) }
}

pub(super) proof fn map_push<A, B>(values: Seq<A>, value: A, project: spec_fn(A) -> B)
    ensures values.push(value).map_values(project) == values.map_values(project).push(project(value)),
{
    assert(values.push(value).map_values(project) =~= values.map_values(project).push(project(value)));
}

impl Evaluation<'_> {
    pub(super) fn formula(&self) -> (formula: &Formula)
        ensures formula@ == self@.formula,
    {
        match &self.formula {
            Cow::Borrowed(formula) => formula,
            Cow::Owned(formula) => formula,
        }
    }

    pub(super) fn into_owned(self) -> (checked: Checked)
        ensures checked@ == checked_value(self@),
    {
        let ghost before = self@;
        let formula = self.formula;
        let scope = self.scope;
        let polarity = self.polarity;
        let dependencies = self.dependencies;
        let statement = Statement::new(into_formula(formula), scope.clone(), polarity);
        let mut copied = Vec::with_capacity(dependencies.len());
        let mut items = dependencies.into_iter();
        let ghost mut consumed = Seq::<InputView>::empty();
        loop
            invariant
                before == self@,
                items.obeys_prophetic_iter_laws(),
                statement@ == statement_value(before),
                before.dependencies == consumed + items.remaining().map_values(|item: Input| item@),
                copied.deep_view() == consumed.map_values(|item: InputView| dependency_value(item)),
            decreases before.dependencies.len() - consumed.len(),
        {
            match items.next() {
                Some(item) => {
                    let ghost value = item@;
                    copied.push(item.into_owned());
                    proof {
                        assert(copied.deep_view() =~= consumed.map_values(
                            |input: InputView| dependency_value(input)).push(dependency_value(value)));
                        assert(consumed.push(value).map_values(|input: InputView| dependency_value(input))
                            =~= consumed.map_values(|input: InputView| dependency_value(input)).push(dependency_value(value)));
                    }
                    proof { consumed = consumed.push(value); }
                },
                None => {
                    proof { assert(consumed =~= before.dependencies); }
                    return Checked::new(statement, copied);
                },
            }
        }
    }

    pub(super) fn matches_statement(&self, statement: &Statement) -> (matches: bool)
        ensures matches == (statement_value(self@) == statement@),
    {
        self.formula() == statement.formula()
            && self.scope == statement.scope() && self.polarity == *statement.polarity()
    }
}

pub open spec fn push_unique_spec<A>(left: Seq<A>, value: A) -> Seq<A> {
    if left.contains(value) { left } else { left.push(value) }
}
pub open spec fn extend_unique_spec<A>(left: Seq<A>, right: Seq<A>) -> Seq<A>
    decreases right.len(),
{
    if right.len() == 0 { left }
    else { extend_unique_spec(push_unique_spec(left, right[0]), right.drop_first()) }
}

pub(super) proof fn extend_push<A>(left: Seq<A>, right: Seq<A>, value: A)
    ensures extend_unique_spec(left, right.push(value)) == push_unique_spec(extend_unique_spec(left, right), value),
    decreases right.len(),
{
    reveal_with_fuel(extend_unique_spec, 3);
    if right.len() > 0 {
        assert(right.push(value).drop_first() == right.drop_first().push(value));
        extend_push(push_unique_spec(left, right[0]), right.drop_first(), value);
    }
}

pub(super) proof fn extend_concat<A>(left: Seq<A>, middle: Seq<A>, right: Seq<A>)
    ensures extend_unique_spec(left, middle + right) == extend_unique_spec(extend_unique_spec(left, middle), right),
    decreases middle.len(),
{
    reveal_with_fuel(extend_unique_spec, 2);
    if middle.len() > 0 {
        assert((middle + right).drop_first() =~= middle.drop_first() + right);
        extend_concat(push_unique_spec(left, middle[0]), middle.drop_first(), right);
    }
}

pub(super) fn push_unique<'a>(left: &mut Vec<Input<'a>>, value: Input<'a>)
    ensures final(left).deep_view() == push_unique_spec(old(left).deep_view(), value@),
{
    let ghost before = left.deep_view();
    proof { assert(before.len() == left.len()); }
    let mut index = 0;
    while index < left.len()
        invariant
            left.deep_view() == before, before.len() == left.len(), index <= left.len(),
            forall|i: int| 0 <= i < index ==> before[i] != value@,
        decreases left.len() - index,
    {
        if left[index] == value {
            proof { assert(before[index as int] == value@); }
            return;
        }
        index += 1;
    }
    left.push(value);
}

pub(super) fn extend_unique<'a>(left: &mut Vec<Input<'a>>, right: Vec<Input<'a>>)
    ensures final(left).deep_view() == extend_unique_spec(old(left).deep_view(), right.deep_view()),
{
    let ghost before = left.deep_view();
    let ghost source = right.deep_view();
    let mut items = right.into_iter();
    let ghost mut consumed = Seq::<InputView>::empty();
    loop
        invariant
            before == old(left).deep_view(), source == right.deep_view(),
            items.obeys_prophetic_iter_laws(),
            source == consumed + items.remaining().map_values(|item: Input| item@),
            left.deep_view() == extend_unique_spec(before, consumed),
        decreases source.len() - consumed.len(),
    {
        match items.next() {
            Some(item) => {
                let ghost value = item@;
                push_unique(left, item);
                proof {
                    extend_push(before, consumed, value);
                    consumed = consumed.push(value);
                }
            },
            None => {
                proof { assert(consumed =~= source); }
                return;
            },
        }
    }
}

pub open spec fn correction_events(rows: Seq<CorrectionView>, target: ClaimRefView, now: int) -> Seq<CorrectionView>
    decreases rows.len(),
{
    if rows.len() == 0 { Seq::empty() }
    else {
        let prior = correction_events(rows.drop_last(), target, now);
        let last = rows.last();
        if last.target == target && last.recorded_at <= now { prior.push(last) } else { prior }
    }
}

proof fn correction_events_step(rows: Seq<CorrectionView>, target: ClaimRefView, now: int, index: int)
    requires 0 <= index < rows.len(),
    ensures correction_events(rows.take(index + 1), target, now) ==
        if rows[index].target == target && rows[index].recorded_at <= now {
            correction_events(rows.take(index), target, now).push(rows[index])
        } else { correction_events(rows.take(index), target, now) },
{
    let prefix = rows.take(index + 1);
    assert(prefix.drop_last() =~= rows.take(index));
    assert(prefix.last() == rows[index]);
    reveal_with_fuel(correction_events, 2);
}

pub(super) fn corrections<'a>(kb: &'a KB, context: Frame<'_>, target: &'a ClaimRef) -> (input: Input<'a>)
    ensures input@ == (InputView::Corrections {
        target: target@, events: correction_events(kb@.corrections, target@, context@.base.now),
    }),
{
    let rows = kb.corrections();
    let mut events = Vec::new();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            rows.deep_view() == kb@.corrections, rows.len() == kb@.corrections.len(),
            index <= rows.len(),
            events.deep_view() == correction_events(kb@.corrections.take(index as int), target@, context@.base.now),
        decreases rows.len() - index,
    {
        proof {
            correction_events_step(kb@.corrections, target@, context@.base.now, index as int);
        }
        let correction = &rows[index];
        if correction.target() == target && correction.recorded_at() <= context.now() {
            events.push(correction);
        }
        index += 1;
    }
    proof { assert(kb@.corrections.take(rows.len() as int) == kb@.corrections); }
    Input::Corrections { target, events }
}

} // verus!
