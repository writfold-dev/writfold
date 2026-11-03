use vstd::prelude::*;

verus! {

#[derive(Clone, Copy)]
pub enum AuthoredProblem {
    AcceptedRevision, Supersession, Actors, Readiness, Acceptor, Executor,
    ExplicitLanded, ResolvableLanded, BoundLanded, LandGate, DeployUseGates,
    CurrentGates, Regression,
}

pub open spec fn problem_text(problem: AuthoredProblem) -> Seq<char> {
    match problem {
        AuthoredProblem::AcceptedRevision => "accepted proposal revision"@,
        AuthoredProblem::Supersession => "resolving acyclic supersession"@,
        AuthoredProblem::Actors => "recorded actors"@,
        AuthoredProblem::Readiness => "review readiness"@,
        AuthoredProblem::Acceptor => "acceptor"@,
        AuthoredProblem::Executor => "executor"@,
        AuthoredProblem::ExplicitLanded => "explicit landed references"@,
        AuthoredProblem::ResolvableLanded => "resolvable landed references"@,
        AuthoredProblem::BoundLanded => "landed references bound to the current target revision"@,
        AuthoredProblem::LandGate => "a land gate"@,
        AuthoredProblem::DeployUseGates => "deploy and use gates"@,
        AuthoredProblem::CurrentGates => "current fully scoped gates"@,
        AuthoredProblem::Regression => "regression note"@,
    }
}

fn problem_label(problem: AuthoredProblem) -> (label: &'static str)
    ensures label@ == problem_text(problem),
{
    match problem {
        AuthoredProblem::AcceptedRevision => "accepted proposal revision",
        AuthoredProblem::Supersession => "resolving acyclic supersession",
        AuthoredProblem::Actors => "recorded actors",
        AuthoredProblem::Readiness => "review readiness",
        AuthoredProblem::Acceptor => "acceptor",
        AuthoredProblem::Executor => "executor",
        AuthoredProblem::ExplicitLanded => "explicit landed references",
        AuthoredProblem::ResolvableLanded => "resolvable landed references",
        AuthoredProblem::BoundLanded => "landed references bound to the current target revision",
        AuthoredProblem::LandGate => "a land gate",
        AuthoredProblem::DeployUseGates => "deploy and use gates",
        AuthoredProblem::CurrentGates => "current fully scoped gates",
        AuthoredProblem::Regression => "regression note",
    }
}

pub open spec fn when_problem(condition: bool, problem: AuthoredProblem) -> Seq<AuthoredProblem> {
    if condition { seq![problem] } else { Seq::empty() }
}

// Both public judgments run the same decision algorithm. The trace is erased;
// boolean-only calls never push onto the empty Vec and therefore never allocate.
pub(super) struct ProblemReducer {
    pub(super) valid: bool,
    pub(super) problems: Vec<&'static str>,
    pub(super) collect: bool,
    pub(super) trace: Ghost<Seq<AuthoredProblem>>,
}

impl ProblemReducer {
    pub(super) open spec fn invariant(&self) -> bool {
        self.valid == (self.trace@.len() == 0)
            && if self.collect {
                self.problems@.len() == self.trace@.len()
                    && forall|i: int| 0 <= i < self.problems@.len() ==>
                        self.problems@[i]@ == problem_text(self.trace@[i])
            } else { self.problems@.len() == 0 }
    }

    pub(super) fn new(collect: bool) -> (result: Self)
        ensures result.invariant(), result.collect == collect, result.trace@ == Seq::<AuthoredProblem>::empty(),
    {
        Self { valid: true, problems: Vec::new(), collect, trace: Ghost(Seq::empty()) }
    }

    pub(super) fn emit(&mut self, problem: AuthoredProblem)
        requires old(self).invariant(),
        ensures final(self).invariant(), final(self).collect == old(self).collect,
            final(self).trace@ == old(self).trace@.push(problem),
    {
        self.valid = false;
        if self.collect { self.problems.push(problem_label(problem)); }
        self.trace = Ghost(self.trace@.push(problem));
    }
}

} // verus!
