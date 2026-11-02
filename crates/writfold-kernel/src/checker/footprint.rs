use super::context::Frame;
use std::borrow::Cow;
use writfold_domain::*;

#[derive(PartialEq)]
pub(super) enum Input<'a> {
    Observation(&'a Observation),
    Claim(&'a Claim),
    Attestation(&'a Attestation),
    Enumeration(&'a Enumeration),
    Corrections {
        target: &'a ClaimRef,
        events: Vec<&'a Correction>,
    },
    Context(Frame<'a>),
}

impl Input<'_> {
    pub(super) fn into_owned(self) -> Dependency {
        match self {
            Self::Observation(value) => Dependency::Observation(value.clone()),
            Self::Claim(value) => Dependency::Claim(value.clone()),
            Self::Attestation(value) => Dependency::Attestation(value.clone()),
            Self::Enumeration(value) => Dependency::Enumeration(value.clone()),
            Self::Corrections { target, events } => Dependency::Corrections {
                target: target.clone(),
                events: events.into_iter().cloned().collect(),
            },
            Self::Context(value) => Dependency::Context(value.normalized()),
        }
    }

    pub(super) fn matches(&self, dependency: &Dependency) -> bool {
        match (self, dependency) {
            (Self::Observation(left), Dependency::Observation(right)) => *left == right,
            (Self::Claim(left), Dependency::Claim(right)) => *left == right,
            (Self::Attestation(left), Dependency::Attestation(right)) => *left == right,
            (Self::Enumeration(left), Dependency::Enumeration(right)) => *left == right,
            (
                Self::Corrections { target, events },
                Dependency::Corrections {
                    target: other,
                    events: values,
                },
            ) => *target == other && events.iter().copied().eq(values.iter()),
            (Self::Context(left), Dependency::Context(right)) => {
                right.now() == &Integer::from(0) && left.same_context(right)
            }
            _ => false,
        }
    }
}

pub(super) struct Evaluation<'a> {
    pub formula: Cow<'a, Formula>,
    pub scope: &'a Scope,
    pub polarity: Polarity,
    pub dependencies: Vec<Input<'a>>,
}

impl Evaluation<'_> {
    pub(super) fn into_owned(self) -> Checked {
        Checked::new(
            Statement::new(self.formula.into_owned(), self.scope.clone(), self.polarity),
            self.dependencies
                .into_iter()
                .map(Input::into_owned)
                .collect(),
        )
    }
    pub(super) fn matches_statement(&self, statement: &Statement) -> bool {
        self.formula.as_ref() == statement.formula()
            && self.scope == statement.scope()
            && self.polarity == *statement.polarity()
    }
}

pub(super) fn extend_unique<'a>(
    left: &mut Vec<Input<'a>>,
    right: impl IntoIterator<Item = Input<'a>>,
) {
    for dependency in right {
        if !left.contains(&dependency) {
            left.push(dependency);
        }
    }
}

pub(super) fn corrections<'a>(kb: &'a KB, context: Frame<'_>, target: &'a ClaimRef) -> Input<'a> {
    Input::Corrections {
        target,
        events: kb
            .corrections()
            .iter()
            .filter(|c| c.target() == target && c.recorded_at() <= context.now())
            .collect(),
    }
}
