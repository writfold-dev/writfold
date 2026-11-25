"""Executable DESIGN MODEL, not Writfold implementation or a host security test.

Run: python dry_run.py --output-dir /tmp/writfold-design-dry-run
Standard library only. Synthetic data, no model/network calls or real effects.
The protected ledger, binding table and intercepted executor are assumptions.
The ten scenarios test consistency of proposed decisions under those assumptions.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass, field
from hashlib import sha256
from itertools import combinations
import json
from pathlib import Path


FULL = frozenset({"context", "model_guard", "effect_guard"})
TOOL_ONLY = frozenset()


def digest(value: str) -> str:
    return sha256(value.encode()).hexdigest()


@dataclass
class Run:
    session: str
    task: str
    policy: str
    workflow: str
    caps: frozenset[str]
    host: str = "adapter-A"
    model: str = "model-A"
    epoch: int = 0
    open: bool = True
    source: dict[str, str] = field(default_factory=dict)
    generation: int = 0
    scope: dict[str, str] | None = None
    scope_generation: int = -1
    scope_complete: bool = False
    reports: dict[str, dict] = field(default_factory=dict)
    completed_generation: int | None = None
    artifact_checks: set[tuple[str, str, str, int]] = field(default_factory=set)


class Model:
    def __init__(self):
        self.runs: dict[str, Run] = {}
        self.bindings: dict[tuple[str, str], str] = {}
        self.prepared: dict[str, dict] = {}
        self.submitted: dict[str, dict] = {}
        self.events: list[dict] = []
        self.checks: list[dict] = []
        self.effects = 0
        self.unmediated_effects = 0
        self.calls = 0

    def event(self, actor, operation, outcome, reason="", **details):
        self.events.append(dict(actor=actor, operation=operation,
                                outcome=outcome, reason=reason, **details))

    def expect(self, actual, expected, meaning):
        passed = actual == expected
        self.checks.append(dict(check=meaning, passed=passed,
                                actual=actual, expected=expected))
        if not passed:
            raise AssertionError(f"{meaning}: {actual!r} != {expected!r}")

    def start(self, session, task, workflow="review", caps=FULL, policy="P1"):
        """Supervisor calls this deterministically, outside the model loop."""
        key = (session, task)
        if key in self.bindings:
            rid = self.bindings[key]
            r = self.runs[rid]
            if not r.open or (r.workflow, r.policy) != (workflow, policy):
                self.event("Writfold", "bind-task", "refused", "binding-conflict")
                return None
            self.event("adapter", "bind-task", "reused", run=rid)
            return rid
        rid = f"run-{len(self.runs) + 1}"
        self.runs[rid] = Run(session, task, policy, workflow, caps)
        self.bindings[key] = rid
        self.event("adapter", "bind-task", "created", run=rid)
        return rid

    def bound(self, session, task, rid):
        r = self.runs.get(rid)
        return bool(r and r.open and self.bindings.get((session, task)) == rid)

    def guarded_action(self, session, task, rid):
        allowed = self.bound(session, task, rid)
        self.event("protected executor", "tool-admission",
                   "allowed" if allowed else "refused",
                   "" if allowed else "no-valid-task-binding")
        if allowed:
            self.effects += 1
        return allowed

    def source(self, rid, names):
        r = self.runs[rid]
        r.source = {name: digest(name + ":version-1") for name in names}
        r.generation += 1
        self.event("source", "set-snapshot", "changed", generation=r.generation)

    def enumerate(self, rid, complete=True, error=False):
        r = self.runs[rid]
        r.scope = dict(r.source) if not error else None
        r.scope_generation = r.generation
        r.scope_complete = complete and not error
        self.event("KB enumerator", "enumerate", "unknown" if error else
                   "complete" if complete else "partial", members=len(r.scope or {}))

    def current_scope(self, rid):
        r = self.runs[rid]
        return (r.scope is not None and r.scope_complete
                and r.scope_generation == r.generation and r.scope == r.source)

    def units(self, rid):
        r = self.runs[rid]
        return {a + "-" + b: {a: r.scope[a], b: r.scope[b]}
                for a, b in combinations(sorted(r.scope or {}), 2)}

    def prepare(self, rid, supplied_skill=True):
        r = self.runs[rid]
        reason = ("missing-observation-capability" if not
                  {"context", "model_guard"} <= r.caps else
                  "incomplete-or-stale-scope" if not self.current_scope(rid) else
                  "required-skill-omitted" if not supplied_skill else "")
        if reason:
            self.event("Writfold", "prepare-admission", "refused", reason)
            return None
        pid = f"prepared-{len(self.prepared) + 1}"
        self.prepared[pid] = dict(run=rid, epoch=r.epoch, policy=r.policy,
                                  generation=r.generation, inputs=dict(r.scope),
                                  skill="skill-v1", model=r.model, host=r.host)
        self.event("adapter", "prepare-request", "admitted", request=pid)
        return pid

    def dispatch(self, rid, pid):
        r, p = self.runs[rid], self.prepared.get(pid)
        valid = bool(p and p["run"] == rid and p["epoch"] == r.epoch
                     and p["policy"] == r.policy and p["generation"] == r.generation
                     and self.bound(r.session, r.task, rid)
                     and {"context", "model_guard"} <= r.caps)
        if not valid:
            self.event("model-call owner", "dispatch", "refused", "prepared-binding-stale")
            return None
        iid = f"inv-{len(self.submitted) + 1}"
        self.calls += 1
        self.submitted[iid] = dict(p)
        self.event("harness observer", "request-submitted", "recorded", invocation=iid)
        return iid

    def report(self, rid, unit, invocation):
        r, evidence = self.runs[rid], self.submitted.get(invocation)
        inputs = self.units(rid).get(unit)
        valid = bool(evidence and inputs and evidence["run"] == rid
                     and evidence["policy"] == r.policy
                     and all(evidence["inputs"].get(k) == v for k, v in inputs.items()))
        if not valid:
            self.event("report validator", "record-report", "refused",
                       "missing-or-mismatched-submission", unit=unit)
            return False
        r.reports[unit] = dict(inputs=inputs, invocation=invocation,
                              kind="model-assessment", policy=r.policy)
        self.event("report validator", "record-report", "recorded", unit=unit)
        return True

    def missing(self, rid):
        r = self.runs[rid]
        return [u for u, inputs in self.units(rid).items()
                if u not in r.reports or r.reports[u]["inputs"] != inputs
                or r.reports[u]["policy"] != r.policy]

    def advance(self, rid):
        r = self.runs[rid]
        reason = ("incomplete-or-stale-scope" if not self.current_scope(rid) else
                  "missing-review-units" if r.workflow == "review" and self.missing(rid)
                  else "")
        if not reason:
            r.completed_generation = r.generation
        self.event("Writfold graph", "advance", "refused" if reason else "advanced",
                   reason, missing=self.missing(rid) if r.workflow == "review" else [])
        return not reason

    def check_draft(self, rid, text, mode="pr-comment"):
        r = self.runs[rid]
        valid = (r.open and self.current_scope(rid)
                 and r.completed_generation == r.generation
                 and mode == "pr-comment" and bool(text) and len(text) <= 240)
        if valid:
            r.artifact_checks.add((digest(text), mode, r.policy, r.generation))
        self.event("delivery checker", "check-exact-artifact",
                   "passed" if valid else "refused", mode=mode)
        return valid

    def publish(self, session, task, rid, text, mode="pr-comment", receiver=False):
        r = self.runs[rid]
        valid = (self.bound(session, task, rid) and (receiver or "effect_guard" in r.caps)
                 and self.current_scope(rid) and r.completed_generation == r.generation
                 and (digest(text), mode, r.policy, r.generation) in r.artifact_checks)
        self.event("protected receiver" if receiver else "harness effect owner",
                   "publish", "allowed" if valid else "refused",
                   "" if valid else "unbound-unsupported-stale-or-unchecked")
        if valid:
            self.effects += 1
        return valid

    def switch(self, rid, host, model, caps):
        r = self.runs[rid]
        r.host, r.model, r.caps = host, model, caps
        r.epoch += 1
        self.event("adapter", "switch-or-compact", "new-context", epoch=r.epoch,
                   host=host, model=model)

    def raw_bypass(self):
        self.unmediated_effects += 1
        self.event("uncontrolled host", "outside-protected-path", "effect-occurred",
                   "Writfold was never consulted; no prevention claim applies")


def setup(m, names="ABCDE", caps=FULL, workflow="review"):
    rid = m.start("session-A", "task-A", workflow=workflow, caps=caps)
    m.source(rid, names)
    m.enumerate(rid)
    return rid


def review(m, rid, omit=()):
    invocation = m.dispatch(rid, m.prepare(rid))
    for unit in m.units(rid):
        if unit not in omit:
            m.report(rid, unit, invocation)
    return invocation


def happy(m):
    rid = setup(m)
    review(m, rid)
    m.expect(m.runs[rid].completed_generation, None, "recording does not advance")
    m.expect(m.advance(rid), True, "ten reports allow graph advancement")
    m.expect(m.check_draft(rid, "Reviewed the declared references; findings attached."), True,
             "exact delivery artifact passes configured checks")
    m.expect(m.publish("session-A", "task-A", rid,
                       "Reviewed the declared references; findings attached."), True,
             "protected sender releases the checked artifact")
    m.expect(m.effects, 1, "one simulated governed effect")


def missing_bootstrap(m):
    m.event("harness", "task-start-hook", "not-called")
    m.expect(m.guarded_action("session-A", "task-A", None), False,
             "independent protected operation check rejects absent run")
    m.expect(m.effects, 0, "no effect occurred through the protected path")
    rid = m.start("session-A", "task-A")
    m.expect(m.guarded_action("session-A", "task-A", rid), True,
             "bootstrap repair establishes the required binding")


def task_identity(m):
    a = m.start("session-A", "task-A")
    replay = m.start("session-A", "task-A")
    m.expect(a, replay, "duplicate start returns the same run")
    b = m.start("session-A", "task-B")
    m.expect(a == b, False, "new task in same session receives a different binding")
    m.expect(m.guarded_action("session-A", "task-B", a), False,
             "task cannot borrow another task's current-run pointer")
    m.runs[b].open = False
    m.expect(m.guarded_action("session-A", "task-B", b), False,
             "closed run cannot authorize new work")
    m.expect(m.start("session-A", "task-A", policy="worker-weakened"), None,
             "replayed bootstrap cannot overwrite admitted policy")


def omitted_skill(m):
    rid = setup(m)
    m.expect(m.prepare(rid, supplied_skill=False), None,
             "omitted skill blocks before model dispatch")
    m.expect(m.calls, 0, "no synthetic model call was made")
    prepared = m.prepare(rid)
    m.expect(m.report(rid, "A-B", prepared), False,
             "prepared context cannot masquerade as submitted invocation")
    invocation = m.dispatch(rid, prepared)
    m.expect(m.report(rid, "A-B", invocation), True,
             "observed submission supports attributable report")


def compaction(m):
    rid = setup(m)
    prepared = m.prepare(rid)
    m.switch(rid, "adapter-A", "model-A", FULL)
    m.expect(m.dispatch(rid, prepared), None,
             "prepared request from prior context epoch cannot be dispatched")
    fresh = m.dispatch(rid, m.prepare(rid))
    m.expect(fresh is not None, True, "fresh preparation restores invocation eligibility")
    m.expect(m.calls, 1, "only the fresh call was submitted")


def partial_scope(m):
    rid = setup(m)
    m.enumerate(rid, complete=False)
    m.expect(m.prepare(rid), None, "partial scope blocks mandatory research input")
    m.enumerate(rid, error=True)
    m.expect(m.advance(rid), False, "search failure does not become empty success")
    m.enumerate(rid)
    m.expect(m.prepare(rid) is not None, True, "complete enumeration allows preparation")


def missing_reviews(m):
    rid = setup(m)
    inv = review(m, rid, omit={"B-D", "C-E"})
    m.expect(m.missing(rid), ["B-D", "C-E"], "diagnostic names exact missing units")
    m.expect(m.advance(rid), False, "eight of ten does not complete review")
    m.report(rid, "B-D", inv)
    m.report(rid, "C-E", inv)
    m.expect(m.advance(rid), True, "all required units now have eligible reports")
    m.expect({r["kind"] for r in m.runs[rid].reports.values()}, {"model-assessment"},
             "coverage does not promote assessments into semantic proofs")


def added_document(m):
    rid = setup(m)
    review(m, rid)
    m.expect(m.advance(rid), True, "five-document review initially complete")
    old = dict(m.runs[rid].source)
    m.source(rid, "ABCDEF")
    m.expect(all(m.runs[rid].source[k] == v for k, v in old.items()), True,
             "old document digests have not changed")
    m.expect(m.advance(rid), False, "new member invalidates whole-scope completeness")
    m.enumerate(rid)
    m.expect(len(m.units(rid)), 15, "six documents require fifteen unordered pairs")
    m.expect(len(m.missing(rid)), 5, "ten unit reports remain reusable under this policy")
    inv = m.dispatch(rid, m.prepare(rid))
    for unit in m.missing(rid):
        m.report(rid, unit, inv)
    m.expect(m.advance(rid), True, "new scope and additional work establish current coverage")


def portability(m):
    rid = setup(m)
    review(m, rid)
    m.switch(rid, "adapter-B", "model-B", FULL)
    m.expect(m.advance(rid), True, "eligible prior reports survive model/host switch")
    m.expect(m.dispatch(rid, m.prepare(rid)) is not None, True,
             "different unconstrained model/host can admit a fresh invocation")
    m.switch(rid, "CLI-only", "opaque-model", TOOL_ONLY)
    m.expect(m.prepare(rid), None, "exact-context requirement is not silently downgraded")
    other = m.start(
        "session-B", "explicit-enumeration-task", workflow="enumerate", caps=TOOL_ONLY)
    m.source(other, "ABCDE")
    m.enumerate(other)
    m.expect(m.advance(other), True, "explicit tool-only contract remains useful")


def delivery_boundary(m):
    rid = setup(m)
    review(m, rid)
    m.advance(rid)
    m.expect(m.report(rid, "A-B", "agent-wrote-trusted-harness"), False,
             "self-authored issuer string is not trusted submission evidence")
    m.check_draft(rid, "Checked draft")
    m.expect(m.publish("session-A", "task-A", rid, "Edited after checking"), False,
             "changed outgoing bytes require a new check")
    m.expect(m.publish("session-A", "task-A", rid, "Checked draft"), True,
             "unchanged checked bytes can pass the protected sender")
    m.switch(rid, "unguarded-host", "model-A", TOOL_ONLY)
    m.expect(m.publish("session-A", "task-A", rid, "Checked draft"), False,
             "adapter cannot claim an unsupported effect interception")
    m.raw_bypass()
    m.expect(m.unmediated_effects, 1,
             "an uncontrolled external path can act without consulting Writfold")
    m.expect(m.publish("session-A", "task-A", rid, "Unchecked bypass output",
                       receiver=True), False,
             "independent receiver refuses unchecked output; it cannot undo prior effects")


SCENARIOS = [
    ("01", "Complete research and delivery", happy, "Allowed", "Preparation, observation, checking and sending have separate owners."),
    ("02", "Startup hook never runs", missing_bootstrap, "Blocked, then repaired", "The protected action path must check binding independently of startup."),
    ("03", "Duplicate start and wrong task", task_identity, "Replay reused; invalid bindings blocked", "Session identity is not task identity; bindings and bootstrap must be explicit."),
    ("04", "Skill omitted or only prepared", omitted_skill, "Blocked before submission", "A prepared packet is not evidence that a model call occurred."),
    ("05", "Compaction before dispatch", compaction, "Stale context blocked; fresh context admitted", "Context changes require fresh preparation and use-time checks."),
    ("06", "Incomplete or failed KB search", partial_scope, "Blocked, then repaired", "The enumerator owns completeness observations; missing input is not empty success."),
    ("07", "Eight of ten comparisons", missing_reviews, "Blocked until two reports added", "Report recording, coverage and graph advancement are distinct."),
    ("08", "A newly related sixth document", added_document, "Old completeness refused; eligible units reused", "Scope membership and per-unit evidence need different dependency sets."),
    ("09", "Model/harness switch and CLI-only use", portability, "Supported work admitted; strict unsupported work blocked", "Capabilities belong to operations; changing host need not discard the KB."),
    ("10", "Forged evidence, altered draft and bypass", delivery_boundary, "Protected path refuses; raw bypass remains possible", "Only the effect owner can enforce; a receiver cannot undo earlier outside effects."),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    results = []
    for sid, title, scenario, outcome, lesson in SCENARIOS:
        m = Model()
        scenario(m)
        # JSON supports lists, not sets, in the recorded expectation values.
        checks = [{k: sorted(v) if isinstance(v, set) else v for k, v in c.items()}
                  for c in m.checks]
        results.append(dict(id=sid, title=title, passed=True, expected_outcome=outcome,
                            lesson=lesson, checks=checks, events=m.events,
                            model_calls=m.calls, protected_effects=m.effects,
                            unmediated_effects=m.unmediated_effects))
    payload = dict(kind="design-model-dry-run", schema_version=1,
                   scope="Synthetic protocol decisions only; no live host or model exercised",
                   assumptions=["Trusted binding table and receipt ledger are protected by assumption",
                                "Source enumeration and request observations are faithful inputs",
                                "Simulated intercepted actions obey the decision atomically",
                                "No cryptographic, OS-isolation or concurrency guarantee is tested"],
                   scenario_count=len(results), assertion_count=sum(len(r["checks"]) for r in results),
                   scenarios=results)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "dry-run-results.json").write_text(json.dumps(payload, indent=2) + "\n")
    rows = "\n".join(f"| {r['id']} | {r['title']} | {r['expected_outcome']} | {r['lesson']} |" for r in results)
    report = f'''# Ten enforcement design dry runs

Executed with the standard-library Python reference model [dry_run.py](dry_run.py).
**{len(results)} scenarios, {payload['assertion_count']} assertions passed.**
Machine-readable events and checks: [dry-run-results.json](dry-run-results.json).

This exercises a proposed protocol under explicit assumptions. It does not run Writfold,
a live model, a real harness, a proof kernel, or a protected production executor. Passing
means the modeled decisions are internally consistent for these traces; it does not
establish that the current product implements them or that a host cannot bypass them.

| # | Scenario | Observed modeled outcome | Design consequence |
|---|---|---|---|
{rows}

## What the dry runs clarified

1. Bootstrap must be ordinary adapter/supervisor code. The model is not responsible for remembering to create a run.
2. Startup automation and enforcement are different. An independent protected operation path must reject a missing binding even when startup never ran.
3. Run binding is per task as well as session/workspace. Replayed startup is idempotent; new tasks, closed runs and mismatched bindings cannot borrow an ambient current-run pointer.
4. Context prepared, request submitted, result recorded and graph advanced are distinct transitions. No receipt may skip those distinctions.
5. The run/scope manifest owns collection completeness; individual reports bind their actual unit inputs. A new document invalidates completeness but can leave earlier pair reports reusable under policy.
6. Missing harness capabilities block the affected strict obligation. Tool-owned evidence and independent receiver checks still support useful portable workflows.
7. An adapter translates and invokes checks. The host or protected receiver owning the effect must actually obey the decision. Scenario 10 intentionally demonstrates that an unmediated external action can happen.

## Responsibilities made explicit

| Component or role | Must do | Must not be assumed to do |
|---|---|---|
| Operator and admitted policy | Select the workflow, baseline requirements, allowed issuers and deployment boundary | Let model classification silently remove baseline obligations |
| Harness or task-entry supervisor | Invoke deterministic bootstrap before governed work; hold calls until admission; own task/session lifecycle | Infer that every user message is a new task, or assume a missing startup callback enforced anything |
| Harness adapter | Translate host events, create/attach the run, bind the task/session, prepare context and submit faithful observations | Decide its own policy, claim stronger evidence than the host exposes, or block an effect it does not control |
| Writfold pure kernels | Judge bindings, obligations, applicability, coverage and graph transitions | Observe a model call, run a tool or physically prevent an external action |
| Writfold CLI and record IO | Persist runs/evidence and publish decisions against the checked state | Treat a worker-editable current-run pointer or issuer string as protected authority |
| Model/agent | Propose plans/actions and produce attributed assessments and drafts | Own mandatory bootstrap, mint trusted receipts, waive gates or mark checked progress by assertion |
| Tools and checkers | Produce version-bound results and observations for operations they actually perform | Prove global semantic relevance or model understanding from structural checks |
| Effect owner or protected receiver | Apply admission at the actual operation and use the checked immutable inputs | Undo earlier effects through uncontrolled paths or infer compliance from the mere existence of a run |

Several roles can live in the same harness process. This is not a proposal for eight
services or a Writfold daemon. In the Python precursor, ordinary wrapper functions can
own bootstrap and dispatch; in another harness, an extension may supply the callbacks.
The protected effect owner must still honor denial and cover the claimed paths.

## One complete interaction sequence

1. The user submits or explicitly resumes a task through the configured entry point.
2. The supervisor calls the adapter; the adapter asks Writfold to create or attach an admissible run and retains the exact task binding. Replayed bootstrap reuses it.
3. Writfold returns current obligations. The supervisor invokes mandatory preparation tools, whose observations are recorded and checked.
4. The adapter assembles required skill/input content. Writfold checks the prepared invocation; the model-call owner dispatches only when allowed.
5. The harness records the actual submitted request and associates returned output/tool proposals with that invocation.
6. The effect owner checks the binding and relevant obligations before each governed tool action, executes allowed work, and reports what happened.
7. The model produces results; tools/checkers validate and record them. Writfold separately checks whether graph advancement is permitted.
8. The delivery checker validates exact bytes; the actual sender checks current authority and publishes only those bytes. Writfold retains the outcome and unresolved work.

Task entry can be a native extension callback or a controlled SDK wrapper. A launch
wrapper only establishes startup unless it also controls later operations. Session start
may not yet contain a task: the binding must be established at an explicit task boundary
or before the first governed action. Compaction is not automatically a new task.

## Reproduce

```sh
python docs/rfcs/2026-11-25-enforcement/dry_run.py --output-dir /tmp/writfold-design-dry-run
```

Outputs are deterministic. All documents, calls, effects and issuer identities are synthetic.
The reference model is a review aid, not a second implementation to keep compatible with
the product. The accepted Rust decisions still require extraction proofs and real adapter
conformance. Dispatch crashes, races, distributed permits, signature validation, natural-
language correctness and actual plugin availability remain outside this exercise.
'''
    (args.output_dir / "dry-runs.md").write_text(report)
    print(json.dumps({k: payload[k] for k in ("scenario_count", "assertion_count", "scope")}))
    for r in results:
        print(f"{r['id']}: PASS | {r['title']} | {r['expected_outcome']}")


if __name__ == "__main__":
    main()
