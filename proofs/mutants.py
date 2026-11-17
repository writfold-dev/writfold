#!/usr/bin/env python3
"""Mutant calibration for the `proofs` gate (docs/writfold/aeneas.md).

Every moved kernel function gets at least one semantic mutation of its Rust
source. For each: copy the crate, apply the mutation, translate it exactly as
the gate does (proofs/extract.sh), build the theorems against the mutated
translation in a scratch copy of the lake project, and require the build to
FAIL inside the named theorem(s) — no other theorem may fail, and the
translation itself must still type-check. A proof that survives its mutant
proves nothing; a failure anywhere else is noise, not calibration.
"""
from dataclasses import dataclass
from pathlib import Path
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PROOFS = ROOT / "proofs"
KERNEL = ROOT / "crates" / "writfold-core"


@dataclass(frozen=True)
class Mutant:
    name: str
    file: str
    before: str
    after: str
    fails: tuple[str, ...]


MUTANTS = (
    # identity is byte-exact: ignoring the last byte is a different identity
    Mutant("same-ignores-last-byte", "bytes.rs",
           "    while i < left.len() {\n        if left[i] != right[i] {",
           "    while i + 1 < left.len() {\n        if left[i] != right[i] {",
           ("Writfold.Core.same_loop_spec",)),
    # run graph: a skipped step is closed, so it is never current
    Mutant("closed-ignores-skip", "plan.rs",
           "    node.done || node.skipped",
           "    node.done",
           ("Writfold.Core.Plan.closed_spec",)),
    # run graph: the current step is the least open one, not the last step
    Mutant("current-returns-last", "plan.rs",
           "        if !closed(&nodes[index]) {\n            return Some(index);",
           "        if !closed(&nodes[index]) {\n            return Some(nodes.len() - 1);",
           ("Writfold.Core.Plan.current_loop_spec",)),
    # run graph: the first owed gate is the first one not met
    Mutant("first-owed-inverted", "plan.rs",
           "        if !met[index] {",
           "        if met[index] {",
           ("Writfold.Core.Plan.first_owed_loop_spec",)),
    # run graph: only the current step may be recorded as done
    Mutant("advance-without-current-check", "plan.rs",
           "    if now != at {\n        return Err(Refusal::NotCurrent(now));",
           "    if false {\n        return Err(Refusal::NotCurrent(now));",
           ("Writfold.Core.Plan.advance_check_spec",)),
    # run graph: every gate of the step must be met
    Mutant("advance-ignores-gates", "plan.rs",
           "        Some(gate) => Err(Refusal::Owed(gate)),",
           "        Some(_) => Ok(()),",
           ("Writfold.Core.Plan.advance_check_spec",)),
    # run graph: a stale step is a done step with an owed gate
    Mutant("stale-ignores-done", "plan.rs",
           "    node.done && first_owed(met).is_some()",
           "    !node.skipped && first_owed(met).is_some()",
           ("Writfold.Core.Plan.stale_spec",)),
    # splice: the new steps go before the current step, not after it
    Mutant("splice-inserts-after-current", "plan.rs",
           "    while index < nodes.len() && index < at {",
           "    while index < nodes.len() && index <= at {",
           ("Writfold.Core.Plan.splice_loop0_spec",)),
    # splice: the new steps are open
    Mutant("splice-new-steps-done", "plan.rs",
           "            done: false,\n            skipped: false,",
           "            done: true,\n            skipped: false,",
           ("Writfold.Core.Plan.splice_loop1_spec",)),
    # splice: every existing step is kept, the last one included
    Mutant("splice-drops-last-step", "plan.rs",
           "    while index < nodes.len() {\n        out.push(Node {",
           "    while index + 1 < nodes.len() {\n        out.push(Node {",
           ("Writfold.Core.Plan.splice_loop2_spec",)),
    # splice: idempotent on the template key
    Mutant("splice-ignores-key", "plan.rs",
           "    if contains(spliced, key) {",
           "    if false {",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # splice: depth is bounded by MAX_DEPTH
    Mutant("splice-depth-off-by-one", "plan.rs",
           "    if depth > MAX_DEPTH {",
           "    if depth > MAX_DEPTH + 1 {",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # splice: the last key spliced counts too
    Mutant("contains-misses-last", "plan.rs",
           "    while index < keys.len() {",
           "    while index + 1 < keys.len() {",
           ("Writfold.Core.Plan.contains_loop_spec",)),
    # splice: an empty plan takes a root splice at 0
    Mutant("splice-check-refuses-empty", "plan.rs",
           "    if nodes.is_empty() {\n        return Ok(Spliced::Insert(0));",
           "    if nodes.is_empty() {\n        return Err(SpliceRefusal::Complete);",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # splice: a complete plan has no current step to splice under
    Mutant("splice-check-admits-complete", "plan.rs",
           "        None => Err(SpliceRefusal::Complete),\n        Some(at) => Ok(Spliced::Insert(at)),",
           "        None => Ok(Spliced::Insert(0)),\n        Some(at) => Ok(Spliced::Insert(at)),",
           ("Writfold.Core.Plan.splice_check_spec",)),
    # gate: a subject that moved while the command ran is never a pass
    Mutant("outcome-ignores-moved", "gate.rs",
           "    if subject_moved {\n        return Outcome::Unknown;",
           "    if false {\n        return Outcome::Unknown;",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: only exit status 0 passes
    Mutant("outcome-any-exit-passes", "gate.rs",
           "        Ending::Exited(0) => Outcome::Pass,",
           "        Ending::Exited(_) => Outcome::Pass,",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: another exit status is a failure, not unknown
    Mutant("outcome-nonzero-unknown", "gate.rs",
           "        Ending::Exited(_) => Outcome::Fail,",
           "        Ending::Exited(_) => Outcome::Unknown,",
           ("Writfold.Core.Gate.outcome_spec",)),
    # gate: a receipt about other bytes in scope never decides the state
    # (the comparison moved into `gate::current`, which `state` calls)
    Mutant("state-ignores-scope", "gate.rs",
           "    evidence.bound && evidence.same_definition && evidence.same_scope",
           "    evidence.bound && evidence.same_definition",
           ("Writfold.Core.Gate.current_spec",)),
    # gate: a receipt about another definition never decides the state
    Mutant("state-ignores-definition", "gate.rs",
           "    evidence.bound && evidence.same_definition && evidence.same_scope",
           "    evidence.bound && evidence.same_scope",
           ("Writfold.Core.Gate.current_spec",)),
    # gate: a receipt from another run or subject, or broken, counts as absent
    Mutant("state-ignores-bound", "gate.rs",
           "        if current(&evidence) {",
           "        if evidence.same_definition && evidence.same_scope {",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: a decision needs a bound receipt (the comparison itself)
    Mutant("current-ignores-bound", "gate.rs",
           "    evidence.bound && evidence.same_definition && evidence.same_scope",
           "    evidence.same_definition && evidence.same_scope",
           ("Writfold.Core.Gate.current_spec",)),
    # gate: with no current receipt, only a bound one makes the gate stale
    Mutant("stale-from-unbound", "gate.rs",
           "        if receipts[index].bound {",
           "        if true {",
           ("Writfold.Core.Gate.newest_bound_loop_spec", "Writfold.Core.Gate.newest_bound_spec")),
    # gate: bound needs this run, this subject and an intact digest
    Mutant("bound-ignores-subject", "gate.rs",
           "    same_run && same_subject && intact",
           "    same_run && intact",
           ("Writfold.Core.Gate.bound_spec",)),
    # gate: a statement from another run or subject never stands
    Mutant("stands-ignores-bound", "gate.rs",
           "    if !said.bound {\n        return false;",
           "    if false {\n        return false;",
           ("Writfold.Core.Gate.stands_spec",)),
    # gate: an unbound attestation is not "some attestation"
    Mutant("attests-ignores-bound", "gate.rs",
           "    said.attested && said.bound",
           "    said.attested",
           ("Writfold.Core.Gate.attests_spec",)),
    # gate: the newest receipt about the current subject decides, not the oldest
    Mutant("state-oldest-first", "gate.rs",
           "    let mut index = receipts.len();\n    while index > 0 {\n        index -= 1;\n        let evidence = receipts[index];",
           "    let mut index = 0;\n    while index < receipts.len() {\n        let evidence = receipts[index];\n        index += 1;",
           ("Writfold.Core.Gate.state_loop_spec", "Writfold.Core.Gate.state_spec")),
    # gate: a failed receipt is a failure
    Mutant("state-failed-satisfies", "gate.rs",
           "                Outcome::Fail => State::Failed,",
           "                Outcome::Fail => State::Satisfied,",
           ("Writfold.Core.Gate.state_loop_spec",)),
    # gate: an attested-class obligation has no checked state to stand on
    Mutant("attested-class-stands-on-check", "gate.rs",
           "        (true, false) => Standing::Checked(State::Open),",
           "        (true, false) => Standing::Checked(state),",
           ("Writfold.Core.Gate.standing_spec",)),
    # gate: a waiver is never a passing check
    Mutant("waiver-satisfies", "gate.rs",
           "        return Standing::Waived;",
           "        return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Gate.standing_spec",)),
    # gate: an attestation is never a passing check
    Mutant("attestation-satisfies", "gate.rs",
           "            return Standing::Attested;",
           "            return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Gate.standing_spec",)),
    # gate: a stale obligation is owed
    Mutant("met-counts-stale", "gate.rs",
           "Standing::Checked(State::Open | State::Stale | State::Failed | State::Unknown)",
           "Standing::Checked(State::Open | State::Failed | State::Unknown)",
           ("Writfold.Core.Gate.met_spec",)),
    # about the present: the recorded definition must be the present one
    Mutant("about-ignores-definition", "gate.rs",
           "    crate::bytes::same(recorded_definition, definition)\n        && crate::bytes::same(recorded_content, content)",
           "    crate::bytes::same(recorded_content, content)",
           ("Writfold.Core.Gate.about_spec",)),
    # a receipt under another definition is not the same definition
    Mutant("evidence-ignores-definition", "gate.rs",
           "        same_definition: crate::bytes::same(&receipt.definition, definition),",
           "        same_definition: true,",
           ("Writfold.Core.Gate.evidence_spec",)),
    # a receipt about other bytes is not about the present content
    Mutant("evidence-ignores-content", "gate.rs",
           "        same_scope: !content.is_empty() && crate::bytes::same(&receipt.content, content),",
           "        same_scope: !content.is_empty(),",
           ("Writfold.Core.Gate.evidence_spec",)),
    # empty present content (unreadable) is never the same content
    Mutant("evidence-accepts-empty-content", "gate.rs",
           "        same_scope: !content.is_empty() && crate::bytes::same(&receipt.content, content),",
           "        same_scope: crate::bytes::same(&receipt.content, content),",
           ("Writfold.Core.Gate.evidence_spec",)),
    # every receipt is judged, the newest included
    Mutant("judge-drops-newest", "gate.rs",
           "    while index < receipts.len() {\n        all.push(",
           "    while index + 1 < receipts.len() {\n        all.push(",
           ("Writfold.Core.Gate.judge_loop_spec",)),
    # an attestation is never a waiver, and a waiver never an attestation
    Mutant("stands-ignores-kind", "gate.rs",
           "    if said.attested != attested {",
           "    if false {",
           ("Writfold.Core.Gate.stands_spec",)),
    # a waiver about other bytes does not stand
    Mutant("statements-counts-moved-waiver", "gate.rs",
           "        if stands(&said[index], false, definition, content) {",
           "        if !said[index].attested {",
           ("Writfold.Core.Gate.statements_loop_spec",)),
    # a gate with no usable definition is never met
    Mutant("advance-undefined-met", "plan.rs",
           "        GateFact::Undefined => false,",
           "        GateFact::Undefined => true,",
           ("Writfold.Core.Plan.fact_met_spec",)),
    # every gate of the step is judged, the last included
    Mutant("met-all-drops-last", "plan.rs",
           "    while index < gates.len() {\n        met.push(",
           "    while index + 1 < gates.len() {\n        met.push(",
           ("Writfold.Core.Plan.met_all_loop_spec",)),
    # a step with no gates owes nothing
    Mutant("advance-refuses-no-gates", "plan.rs",
           "    advance_check(nodes, at, &met_all(gates))",
           "    if gates.is_empty() {\n        return Err(Refusal::Owed(0));\n    }\n    advance_check(nodes, at, &met_all(gates))",
           ("Writfold.Core.Plan.advance_facts_spec", "Writfold.Core.Plan.advance_facts_ok_iff")),
    # checklist: a supported row covers only when it cites something
    Mutant("item-accepts-no-evidence", "checklist.rs",
           "            if row.evidence > 0 && row.grounded {\n                Item::Supported",
           "            if row.grounded {\n                Item::Supported",
           ("Writfold.Core.Checklist.item_spec",)),
    # checklist: a supported row covers only while its evidence is grounded
    Mutant("item-accepts-ungrounded", "checklist.rs",
           "            if row.evidence > 0 && row.grounded {\n                Item::Supported",
           "            if row.evidence > 0 {\n                Item::Supported",
           ("Writfold.Core.Checklist.item_spec",)),
    # checklist: a row about other content or another definition is stale
    Mutant("item-accepts-not-current", "checklist.rs",
           "    if !row.current {\n        return Item::Stale;",
           "    if false {\n        return Item::Stale;",
           ("Writfold.Core.Checklist.item_spec", "Writfold.Core.Checklist.item_none_found_iff")),
    # checklist: an unsupported item is not covered
    Mutant("covered-counts-unsupported", "checklist.rs",
           "        Item::Open | Item::Unsupported | Item::Stale => false,",
           "        Item::Unsupported => true,\n        Item::Open | Item::Stale => false,",
           ("Writfold.Core.Checklist.covered_under_spec",)),
    # checklist: not-applicable covers only where the policy accepts it
    Mutant("covered-ignores-na", "checklist.rs",
           "        Item::NotApplicable => na,",
           "        Item::NotApplicable => true,",
           ("Writfold.Core.Checklist.covered_under_spec",)),
    # checklist: none-found covers only where the policy accepts it
    Mutant("covered-none-found-without-policy", "checklist.rs",
           "        Item::NoneFound => none,",
           "        Item::NoneFound => true,",
           ("Writfold.Core.Checklist.covered_under_spec",)),
    # checklist: none-found needs at least one cited look
    Mutant("item-none-found-no-evidence", "checklist.rs",
           "            if row.evidence > 0 && row.grounded {\n                Item::NoneFound",
           "            if row.grounded {\n                Item::NoneFound",
           ("Writfold.Core.Checklist.item_spec", "Writfold.Core.Checklist.item_none_found_iff")),
    # checklist: only required items are owed
    # (`rollup_under` reads `required` through `owes`)
    Mutant("rollup-ignores-required", "checklist.rs",
           "    entry.required && !covered_under(entry.item, entry.na, entry.none)",
           "    !covered_under(entry.item, entry.na, entry.none)",
           ("Writfold.Core.Checklist.owes_spec",)),
    # checklist: every entry is looked at, the last included
    Mutant("first-owed-skips-last", "checklist.rs",
           "    while index < entries.len() {\n        if owes(",
           "    while index + 1 < entries.len() {\n        if owes(",
           ("Writfold.Core.Checklist.first_owed_loop_spec",)),
    # checklist: an optional stale item does not make the checklist stale
    Mutant("stale-ignores-required", "checklist.rs",
           "    entry.required && matches!(entry.item, Item::Stale)",
           "    matches!(entry.item, Item::Stale)",
           ("Writfold.Core.Checklist.stale_spec",)),
    # checklist: a stale checklist is not attested
    Mutant("checklist-attested-when-stale", "checklist.rs",
           "        attested_current: rollup.covered,",
           "        attested_current: rollup.covered || rollup.stale,",
           ("Writfold.Core.Checklist.standing_spec",)),
    # delivery: an empty set of obligations is not met
    Mutant("delivery-empty-met", "delivery.rs",
           "    if obligations.is_empty() {\n        return Standing::Checked(State::Open);",
           "    if obligations.is_empty() {\n        return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Delivery.standing_spec",)),
    # delivery: a waived obligation is never read as a passing check
    Mutant("delivery-waived-satisfied", "delivery.rs",
           "    if waived {\n        Standing::Waived",
           "    if false {\n        Standing::Waived",
           ("Writfold.Core.Delivery.standing_loop_spec",)),
    # delivery: an unmet obligation stops the delivery
    Mutant("delivery-unmet-ignored", "delivery.rs",
           "    !standing.met()",
           "    false",
           ("Writfold.Core.Delivery.unmet_spec",)),
    # delivery: a bound candidate never decides an open step
    Mutant("deciding-ignores-bound", "delivery.rs",
           "        if !bound[index] {",
           "        if true {",
           ("Writfold.Core.Delivery.deciding_loop_spec", "Writfold.Core.Delivery.deciding_spec")),
    # delivery: the newest unbound candidate decides, not the oldest
    Mutant("deciding-first-unbound", "delivery.rs",
           "    let mut index = bound.len();\n    while index > 0 {\n        index -= 1;\n        if !bound[index] {",
           "    let mut index = 0;\n    while index < bound.len() {\n        index += 1;\n        if !bound[index - 1] {",
           ("Writfold.Core.Delivery.deciding_loop_spec", "Writfold.Core.Delivery.deciding_spec")),
    # delivery: a mode that declares nothing is never ready
    Mutant("ready-accepts-empty", "delivery.rs",
           "    standing(obligations).met()",
           "    obligations.is_empty() || standing(obligations).met()",
           ("Writfold.Core.Delivery.ready_iff",)),
    # delivery: a passed check whose cited evidence moved is stale
    Mutant("rechecked-keeps-satisfied", "delivery.rs",
           "    if matches!(checked, State::Satisfied) && moved > 0 {",
           "    if false {",
           ("Writfold.Core.Delivery.rechecked_spec",)),
    # delivery guard: unknown bytes need at least one candidate
    Mutant("guard-allows-no-candidate", "delivery.rs",
           "        Body::Unknown => candidates > 0 && all_ready,",
           "        Body::Unknown => all_ready,",
           ("Writfold.Core.Delivery.guard_allow_iff",)),
    # delivery guard: known bytes pass only when a ready candidate holds them
    Mutant("guard-known-any-ready", "delivery.rs",
           "        Body::Known => same_ready,",
           "        Body::Known => same_ready || all_ready,",
           ("Writfold.Core.Delivery.guard_allow_iff",)),
    # guard: an unknown fact is never met
    Mutant("all-met-accepts-unknown", "guard.rs",
           "    matches!(fact, Fact::Met)",
           "    !matches!(fact, Fact::Unmet)",
           ("Writfold.Core.Guard.met_spec",)),
    # guard: every requirement is looked at, the last included
    Mutant("all-met-skips-last", "guard.rs",
           "    while i < facts.len() {\n        if !met(",
           "    while i + 1 < facts.len() {\n        if !met(",
           ("Writfold.Core.Guard.all_met_loop_spec",)),
    # guard: a matched rule passes only when every requirement is met
    Mutant("decide-ignores-facts", "guard.rs",
           "    verdict(all_met(facts), action)",
           "    verdict(true, action)",
           ("Writfold.Core.Guard.decide_spec",)),
    # guard: a deny rule denies
    Mutant("verdict-deny-asks", "guard.rs",
           "        Action::Deny => Decision::Deny,\n        Action::Ask => Decision::Ask,\n    }\n}\n\n/// `Pass` iff",
           "        Action::Deny => Decision::Ask,\n        Action::Ask => Decision::Ask,\n    }\n}\n\n/// `Pass` iff",
           ("Writfold.Core.Guard.verdict_spec",)),
    # guard: a denial wins over a question
    Mutant("combine-ask-over-deny", "guard.rs",
           "    if matches!(a, Decision::Deny) || matches!(b, Decision::Deny) {\n        return Decision::Deny;\n    }\n    if matches!(a, Decision::Ask) || matches!(b, Decision::Ask) {\n        return Decision::Ask;\n    }",
           "    if matches!(a, Decision::Ask) || matches!(b, Decision::Ask) {\n        return Decision::Ask;\n    }\n    if matches!(a, Decision::Deny) || matches!(b, Decision::Deny) {\n        return Decision::Deny;\n    }",
           ("Writfold.Core.Guard.combine_spec",)),
    # guard: an audit rule never decides the call
    Mutant("effect-enforces-audit", "guard.rs",
           "        Mode::Audit => Decision::Pass,",
           "        Mode::Audit => decided,",
           ("Writfold.Core.Guard.effect_spec",)),
    # guard: gates.met is not met by an empty set
    Mutant("gates-fact-met-on-nothing", "guard.rs",
           "    if owed > 0 && unmet == 0 && invalid == 0 {",
           "    if unmet == 0 && invalid == 0 {",
           ("Writfold.Core.Guard.gates_fact_met_iff",)),
    # guard: an invalid receipt or definition keeps gates.met unmet
    Mutant("gates-fact-ignores-invalid", "guard.rs",
           "    if owed > 0 && unmet == 0 && invalid == 0 {",
           "    if owed > 0 && unmet == 0 {",
           ("Writfold.Core.Guard.gates_fact_met_iff",)),
    # guard: the Stop nudge never fires while a stop hook is continuing
    Mutant("nudge-while-active", "guard.rs",
           "    !stop_hook_active && !already_nudged && !all_met(facts)",
           "    !already_nudged && !all_met(facts)",
           ("Writfold.Core.Guard.nudge_spec",)),
    # guard: a turn rule is not held back by the session flag
    Mutant("nudge-with-ignores-repeat", "guard.rs",
           "        already_nudged && matches!(repeat, Repeat::Session),",
           "        already_nudged,",
           ("Writfold.Core.Guard.nudge_with_iff",)),
    # guard: every matched rule counts, the last included
    Mutant("decide-all-drops-last", "guard/call.rs",
           "    while index < rules.len() {\n        decided = combine",
           "    while index + 1 < rules.len() {\n        decided = combine",
           ("Writfold.Core.Guard.decide_all_loop_spec",)),
    # guard: a pass on a waiver is logged
    Mutant("logged-drops-waiver", "guard/call.rs",
           "    matches!(mode, Mode::Audit) || !matches!(decided, Decision::Pass) || waived",
           "    matches!(mode, Mode::Audit) || !matches!(decided, Decision::Pass)",
           ("Writfold.Core.Guard.logged_iff",)),
    # guard: run.current needs an open run
    Mutant("run-current-closed", "guard/require.rs",
           "    if found && open {",
           "    if found {",
           ("Writfold.Core.Guard.run_current_met_iff",)),
    # guard: step.met is never met when the plan could not be read
    Mutant("step-fact-met-unread", "guard/require.rs",
           "        Looked::Failed => Fact::Unknown,",
           "        Looked::Failed => Fact::Met,",
           ("Writfold.Core.Guard.step_fact_met_iff", "Writfold.Core.Guard.step_fact_unknown_iff")),
    # guard: an undefined gate is never met
    Mutant("gate-fact-undefined-met", "guard/require.rs",
           "            if defined && met {",
           "            if met {",
           ("Writfold.Core.Guard.gate_fact_met_iff",)),
    # guard: gates that could not be judged are unknown
    Mutant("gates-read-unread-counts", "guard/require.rs",
           "        Looked::No | Looked::Failed => Fact::Unknown,\n        Looked::Read => super::gates_fact(owed, unmet, invalid),",
           "        Looked::No => Fact::Unknown,\n        Looked::Failed | Looked::Read => super::gates_fact(owed, unmet, invalid),",
           ("Writfold.Core.Guard.gates_read_met_iff",)),
    # grounding: a waiver is never evidence
    Mutant("waiver-grounds", "ground.rs",
           "        Standing::Checked(_) | Standing::Waived => Grounding::Ungrounded,",
           "        Standing::Checked(_) => Grounding::Ungrounded,\n        Standing::Waived => Grounding::Grounded,",
           ("Writfold.Core.Ground.of_gate_spec",)),
    # grounding: a reference to no gate grounds nothing
    Mutant("of-gate-ignores-found", "ground.rs",
           "    if !found {\n        return Grounding::Ungrounded;",
           "    if false {\n        return Grounding::Ungrounded;",
           ("Writfold.Core.Ground.of_gate_spec",)),
    # grounding: a receipt about other content does not ground
    Mutant("receipt-ignores-content", "ground.rs",
           "        && facts.same_content\n",
           "",
           ("Writfold.Core.Ground.of_receipt_grounded_iff",)),
    # grounding: a receipt grounds only while its gate stands met
    Mutant("receipt-ignores-gate-met", "ground.rs",
           "        && facts.gate_met\n",
           "\n",
           ("Writfold.Core.Ground.of_receipt_grounded_iff",)),
    # grounding: a retrieval with no citation grounds nothing
    Mutant("citations-zero-grounds", "ground.rs",
           "    if count > 0 && not_fresh == 0 {",
           "    if not_fresh == 0 {",
           ("Writfold.Core.Ground.of_citations_grounded_iff",)),
    # grounding: a token grounds only as exactly one citation
    Mutant("token-many-grounds", "ground.rs",
           "    if count == 1 && not_fresh == 0 {",
           "    if count > 0 && not_fresh == 0 {",
           ("Writfold.Core.Ground.of_token_grounded_iff",)),
    # grounding: an observation with no citation grounds nothing
    Mutant("observation-no-citation-grounds", "ground.rs",
           "    if grounds(status, count > 0 && not_fresh == 0) {",
           "    if grounds(status, not_fresh == 0) {",
           ("Writfold.Core.Ground.of_observation_grounded_iff",)),
    # grounding: only the current attestation is attested
    Mutant("statement-not-current-attested", "ground.rs",
           "    if current_attestation {",
           "    if true {",
           ("Writfold.Core.Ground.of_statement_spec",)),
    # grounding: a changed standing word is a moved reference
    Mutant("moved-ignores-word", "ground.rs",
           "    matches!(now, Grounding::Ungrounded) || !same_word",
           "    matches!(now, Grounding::Ungrounded)",
           ("Writfold.Core.Ground.moved_iff",)),
    # observations: a partial look never grounds
    Mutant("observation-partial-grounds", "observe.rs",
           "        Status::Ok => cites_fresh,\n        Status::Partial\n        | Status::NoData",
           "        Status::Ok | Status::Partial => cites_fresh,\n        Status::NoData",
           ("Writfold.Core.Ground.grounds_iff",)),
    # observations: a partial look embeds its citation
    Mutant("embeds-drops-partial", "observe.rs",
           "        Status::Ok | Status::Partial => true,",
           "        Status::Ok => true,\n        Status::Partial => false,",
           ("Writfold.Core.Ground.embeds_iff",)),
    # plan: a step locked by the run's pinned policy cannot be skipped
    Mutant("skip-without-lock", "plan.rs",
           "    if locked {\n        return Err(SkipRefusal::Locked);",
           "    if false {\n        return Err(SkipRefusal::Locked);",
           ("Writfold.Core.Plan.skip_check_spec",)),
    # plan: a skip record of a locked step never closes it
    Mutant("skipped-ignores-lock", "plan.rs",
           "    recorded && !locked",
           "    recorded",
           ("Writfold.Core.Plan.skipped_spec", "Writfold.Core.Plan.locked_skip_never_closes")),
    # plan: the last step closes only with every required template present
    Mutant("close-check-ignores-present", "plan.rs",
           "    first_owed(present)\n}",
           "    None\n}",
           ("Writfold.Core.Plan.close_check_spec",)),
    # plan: a gate is judged under its own policy
    Mutant("fact-met-ignores-policy", "plan.rs",
           "        GateFact::Stands(standing, accept) => crate::policy::met(*standing, accept),",
           "        GateFact::Stands(standing, _) => standing.met(),",
           ("Writfold.Core.Plan.fact_met_spec",)),
    # policy: a gate that rejects waivers is never met by one
    Mutant("met-ignores-waived", "policy.rs",
           "        Standing::Waived => accept.waived,",
           "        Standing::Waived => true,",
           ("Writfold.Core.Policy.met_spec",)),
    # policy: a policy without accept is exactly today's rule
    Mutant("met-default-counts-stale", "policy.rs",
           "        Standing::Checked(state) => matches!(state, State::Satisfied) && accept.satisfied,",
           "        Standing::Checked(state) => {\n            (matches!(state, State::Satisfied) || (matches!(state, State::Stale) && accept.waived))\n                && accept.satisfied\n        }",
           ("Writfold.Core.Policy.met_spec", "Writfold.Core.Policy.met_default")),
    # policy: a pinned digest with nothing there now is moved, never same
    Mutant("pin-one-missing-same", "policy.rs",
           "            None => Pin::Moved,",
           "            None => Pin::Same,",
           ("Writfold.Core.Policy.pin_one_spec", "Writfold.Core.Policy.pin_one_same_iff")),
    # policy: an empty pin is not "same"
    Mutant("pin-all-empty-same", "policy.rs",
           "    if pins.is_empty() || any_unpinned(pins) {",
           "    if any_unpinned(pins) {",
           ("Writfold.Core.Policy.pin_all_spec",)),
    # policy: one moved pin moves the run
    Mutant("pin-all-misses-moved", "policy.rs",
           "        if is_moved(&pins[index]) {\n            return true;",
           "        if false {\n            return true;",
           ("Writfold.Core.Policy.any_moved_loop_spec",)),
    # policy: a waived gate is never read as passed
    Mutant("close-waived-passed", "policy.rs",
           "    if any_waived(met) {\n        return Close::Waived;",
           "    if any_waived(met) {\n        return Close::Passed;",
           ("Writfold.Core.Policy.close_spec",)),
    # policy: a moved policy is never read as complete
    Mutant("summary-skips-moved", "policy.rs",
           "    if is_moved(&facts.pin) {\n        return Summary::PolicyMoved;",
           "    if false {\n        return Summary::PolicyMoved;",
           ("Writfold.Core.Policy.summary_spec",)),
    # policy: a closed run with an open step is closed-incomplete
    Mutant("summary-ignores-closed", "policy.rs",
           "        if facts.run_closed {\n            return Summary::ClosedIncomplete;",
           "        if false {\n            return Summary::ClosedIncomplete;",
           ("Writfold.Core.Policy.summary_spec",)),
    # policy: a skipped, waived or attested step is never plain complete
    Mutant("summary-complete-with-skip", "policy.rs",
           "    if facts.repinned || !all_passed(closes) {",
           "    if facts.repinned {",
           ("Writfold.Core.Policy.summary_spec",)),
    # scope: an exclusion that matches keeps the path out
    Mutant("covers-ignores-exclusion", "scope.rs",
           "        if patterns[index].exclude && hit {\n            return false;",
           "        if false && hit {\n            return false;",
           ("Writfold.Core.Scope.covers_loop_spec",)),
    # scope: a scope with no pattern that includes covers nothing
    Mutant("covers-empty-includes", "scope.rs",
           "    let mut included = false;",
           "    let mut included = true;",
           ("Writfold.Core.Scope.covers_iff",)),
    # scope: an including pattern counts only when it matches
    Mutant("include-ignores-hit", "scope.rs",
           "    included || (!exclude && hit)",
           "    included || !exclude",
           ("Writfold.Core.Scope.include_spec",)),
    # scope: a `*` that led nowhere stretches by one more character
    Mutant("segment-no-backtrack", "scope.rs",
           "    if c.i < pattern.len() && same_char(pattern[c.i], text[c.j]) {\n"
           "        return Next::Go(Cursor {\n            i: c.i + 1,\n            j: c.j + 1,\n"
           "            starred: c.starred,\n            at: c.at,\n            from: c.from,\n"
           "        });\n    }\n    if c.starred {",
           "    if c.i < pattern.len() && same_char(pattern[c.i], text[c.j]) {\n"
           "        return Next::Go(Cursor {\n            i: c.i + 1,\n            j: c.j + 1,\n"
           "            starred: c.starred,\n            at: c.at,\n            from: c.from,\n"
           "        });\n    }\n    if false {",
           ("Writfold.Core.Scope.char_step_spec",)),
    # scope: `*` matches nothing too, at the end of a segment included
    Mutant("star-needs-a-char", "scope.rs",
           "    if c.i < pattern.len() && pattern[c.i] == STAR {",
           "    if c.i < pattern.len() && pattern[c.i] == STAR && c.j < text.len() {",
           ("Writfold.Core.Scope.char_step_spec",)),
    # scope: `?` matches any one character
    Mutant("same-char-ignores-any", "scope.rs",
           "    pattern == ANY || pattern == text",
           "    (pattern == ANY && text == ANY) || pattern == text",
           ("Writfold.Core.Scope.same_char_spec",)),
    # scope: only `**` is a segment wildcard, not `*` followed by anything
    Mutant("globstar-one-star", "scope.rs",
           "    segment.len() == 2 && segment[0] == STAR && segment[1] == STAR",
           "    segment.len() == 2 && segment[0] == STAR",
           ("Writfold.Core.Scope.globstar_spec",)),
    # scope: `**` matches no segment too, at the end of a path included
    Mutant("globstar-needs-a-segment", "scope.rs",
           "    if c.i < wanted.len() && globstar(&wanted[c.i]) {",
           "    if c.i < wanted.len() && globstar(&wanted[c.i]) && c.j < parts.len() {",
           ("Writfold.Core.Scope.path_step_spec",)),
    # scope: a path with segments left over after the pattern does not match
    Mutant("segments-accept-trailing", "scope.rs",
           "    if c.j >= parts.len() {",
           "    if c.j >= parts.len() || c.i >= wanted.len() {",
           ("Writfold.Core.Scope.path_step_spec",)),
    # plan writes: a step write stands only while the step is still current
    Mutant("recheck-current-ignored", "plan/recheck.rs",
           "            if is_current {\n                None",
           "            if true {\n                None",
           ("Writfold.Core.Recheck.still_stands_spec",)),
    # plan writes: a splice stands only while its ordinal is still the next one
    Mutant("recheck-ordinal-ignored", "plan/recheck.rs",
           "            if next != *ordinal {",
           "            if false {",
           ("Writfold.Core.Recheck.still_stands_spec",)),
    # plan writes: a root end template is not under a current step
    Mutant("recheck-root-needs-current", "plan/recheck.rs",
           "            if *parent_current && !is_current {",
           "            if !is_current {",
           ("Writfold.Core.Recheck.still_stands_spec",)),
    # plan writes: a write on an unplanned run stands only while no plan was set
    Mutant("recheck-ignores-plan", "plan/recheck.rs",
           "            if planned {\n                Some(Moved::Planned)",
           "            if false {\n                Some(Moved::Planned)",
           ("Writfold.Core.Recheck.still_stands_spec",)),
    # plan writes: the sidecar's state wins over the manifest's
    Mutant("writable-manifest-wins", "plan/recheck.rs",
           "    match sidecar_closed {",
           "    match manifest_closed {",
           ("Writfold.Core.Recheck.writable_spec", "Writfold.Core.Recheck.sidecar_wins",
            "Writfold.Core.Recheck.closed_refuses")),
    # decisions: accepting needs readiness
    Mutant("accepted-without-readiness", "decision.rs",
           "            State::Proposed => Some(needs(true, true, false, false, false)),",
           "            State::Proposed => Some(needs(false, true, false, false, false)),",
           ("Writfold.Core.Decision.step_spec",)),
    # decisions: verifying needs grounded evidence
    Mutant("verified-without-evidence", "decision.rs",
           "            State::Applied => Some(needs(false, false, false, true, false)),",
           "            State::Applied => Some(needs(false, false, false, false, false)),",
           ("Writfold.Core.Decision.step_spec",)),
    # decisions: a superseded decision is not superseded again
    Mutant("superseded-twice", "decision.rs",
           "            State::Superseded => None,",
           "            State::Superseded => Some(needs(false, false, false, false, true)),",
           ("Writfold.Core.Decision.step_spec",)),
    # decisions: landing is never written
    Mutant("landing-written", "decision.rs",
           "        State::Landing | State::Regressed | State::Other => None,",
           "        State::Landing => Some(needs(false, false, false, false, false)),\n"
           "        State::Regressed | State::Other => None,",
           ("Writfold.Core.Decision.step_spec",)),
    # investigations: a missing section stops the conclusion
    Mutant("conclude-ignores-section", "investigation.rs",
           "        checked_against: !checked_against,",
           "        checked_against: false,",
           ("Writfold.Core.Decision.conclude_spec",)),
    # investigations: every hypothesis is counted, the last included
    Mutant("conclude-skips-last", "investigation.rs",
           "    while index < hypotheses.len() {",
           "    while index + 1 < hypotheses.len() {",
           ("Writfold.Core.Decision.conclude_loop_spec",)),
    # investigations: an open hypothesis stops the conclusion
    Mutant("none-ignores-open", "investigation.rs",
           "        !self.checked_against && !self.scope_and_limits && self.open == 0",
           "        !self.checked_against && !self.scope_and_limits",
           ("Writfold.Core.Decision.none_spec",)),
    # verdict: a span found at other lines is moved, never fresh
    Mutant("moved-span-fresh", "verdict.rs",
           "            Located::At(_, _) => Verdict::Moved,",
           "            Located::At(_, _) => Verdict::Fresh,",
           ("Writfold.Core.Verdict.observed_spec",)),
    # verdict: a file too long to search is changed, never fresh
    Mutant("unsearched-fresh", "verdict.rs",
           "            Located::Absent | Located::Unsearched => Verdict::Changed,",
           "            Located::Absent => Verdict::Changed,\n            Located::Unsearched => Verdict::Fresh,",
           ("Writfold.Core.Verdict.observed_spec",)),
    # verdict: a mark never hides a change
    Mutant("mark-hides-change", "verdict.rs",
           "        Verdict::Changed | Verdict::Gone | Verdict::Expired => reached,",
           "        Verdict::Gone | Verdict::Expired => reached,",
           ("Writfold.Core.Verdict.with_mark_spec",)),
    # verdict: the protocol first, then the kind, then the operation
    Mutant("handshake-kind-as-op", "verdict.rs",
           "            } else if !d.lists_kind {\n                Handshake::Kind",
           "            } else if !d.lists_kind {\n                Handshake::Op",
           ("Writfold.Core.Verdict.handshake_spec",)),
    # verdict: a reply counts only after an agreed handshake
    Mutant("exchange-ignores-handshake", "verdict.rs",
           "    if !matches!(handshake, Handshake::Agreed) || !clean_exit {",
           "    if !clean_exit {",
           ("Writfold.Core.Verdict.exchange_said_iff", "Writfold.Core.Verdict.exchange_spec",)),
    # verdict: exactly one reply counts, not the first of two
    Mutant("exchange-first-of-two", "verdict.rs",
           "    if replies.len() != 1 {",
           "    if replies.is_empty() {",
           ("Writfold.Core.Verdict.exchange_said_iff", "Writfold.Core.Verdict.exchange_spec")),
    # verdict: a reply whose age cannot be read is expired
    Mutant("aged-keeps-unreadable", "verdict.rs",
           "            None => Checker::Expired,\n        },",
           "            None => cached,\n        },",
           ("Writfold.Core.Verdict.aged_spec",)),
    # verdict: the token's own ttl comes before the kind's default
    Mutant("lifetime-token-as-default", "verdict.rs",
           "        Some(ttl) => Lifetime::Token(ttl),",
           "        Some(ttl) => Lifetime::Default(ttl),",
           ("Writfold.Core.Verdict.lifetime_spec",)),
    # verdict: far past the end of i64, the sign alone decides expiry
    Mutant("expired-overflow-kept", "verdict.rs",
           "    if seen < 0 && now > i64::MAX + seen {\n        return true;",
           "    if seen < 0 && now > i64::MAX + seen {\n        return false;",
           ("Writfold.Core.Verdict.expired_spec",)),
    # verdict: the cited lines must lie inside the file
    Mutant("window-cited-off-by-one", "verdict/locate.rs",
           "    if last <= lines {",
           "    if last < lines {",
           ("Writfold.Core.Verdict.window_spec",)),
    # verdict: the cited window comes before any other
    Mutant("place-ignores-cited", "verdict/locate.rs",
           "    if cited {\n        return Place::Same;",
           "    if false {\n        return Place::Same;",
           ("Writfold.Core.Verdict.place_same_iff", "Writfold.Core.Verdict.place_at_iff", "Writfold.Core.Verdict.place_absent_iff",)),
    # check receipts: an unverifiable answer is never shared
    Mutant("recordable-unverifiable", "verdict/recorded.rs",
           "        Checker::Said(said) => !matches!(said, Said::Unverifiable),",
           "        Checker::Said(_) => true,",
           ("Writfold.Core.Recorded.recordable_iff",)),
    # check receipts: a date more than an hour ahead is unusable
    Mutant("recorded-age-no-skew", "verdict/recorded.rs",
           "    if now <= i64::MAX - CLOCK_SKEW && checked_at > now + CLOCK_SKEW {",
           "    if now <= i64::MAX - CLOCK_SKEW && checked_at > now {",
           ("Writfold.Core.Recorded.recorded_age_spec",)),
    # check receipts: the newer statement comes first
    Mutant("before-prefers-older", "verdict/recorded.rs",
           "        return a.age < b.age;",
           "        return a.age > b.age;",
           ("Writfold.Core.Recorded.before_spec",)),
    # check receipts: on a tie the first receipt is kept
    Mutant("better-tie-takes-later", "verdict/recorded.rs",
           "            if before(&recorded[index], &recorded[b]) {",
           "            if !before(&recorded[b], &recorded[index]) {",
           ("Writfold.Core.Recorded.better_spec",)),
    # check receipts: on a full tie a receipt wins over the local statement
    Mutant("newest-local-on-tie", "verdict/recorded.rs",
           "                if before(&s, &recorded[i]) {",
           "                if !before(&recorded[i], &s) {",
           ("Writfold.Core.Recorded.newest_spec",)),
    # walk: a skipped edge keeps coverage partial
    Mutant("complete-ignores-skipped", "walk.rs",
           "    frontier == 0 && skipped == 0",
           "    frontier == 0",
           ("Writfold.Core.Walk.complete_iff",)),
    # lint: a warning blocks only under --deny-warnings
    Mutant("blocks-any-warning", "diag/mod.rs",
           "    errors > 0 || (deny && warnings > 0)",
           "    errors > 0 || warnings > 0",
           ("Writfold.Core.Walk.blocks_iff",)),
    # find: a record is shown once, under the first tier that found it
    Mutant("admit-shows-twice", "find.rs",
           "    shown < limit && !seen && passes",
           "    shown < limit && passes",
           ("Writfold.Core.Walk.admit_iff",)),
    # command: --help is exactly those bytes
    Mutant("is-help-wrong-byte", "command/mod.rs",
           "        && word[1] == b'-'",
           "        && word[1] == b'h'",
           ("Writfold.Core.Command.is_help_spec",)),
    # command: every word of a rule's prefix is compared, the last included
    Mutant("named-skips-last", "command/mod.rs",
           "    while i < prefix.len() {\n        if !crate::bytes::same(&prefix[i], &words[i]) {",
           "    while i + 1 < prefix.len() {\n        if !crate::bytes::same(&prefix[i], &words[i]) {",
           ("Writfold.Core.Command.named_loop_spec",)),
    # command: --help stops a match only when it is the one word after the prefix
    Mutant("named-help-not-alone", "command/mod.rs",
           "    !(words.len() - prefix.len() == 1 && is_help(&words[prefix.len()]))",
           "    !(words.len() - prefix.len() >= 1 && is_help(&words[prefix.len()]))",
           ("Writfold.Core.Command.named_loop_spec",)),
    # find: a one-token query needs four characters
    Mutant("query-ok-three-chars", "find.rs",
           "    query.len() >= 2 || (query.len() == 1 && query[0].len() >= 4)",
           "    query.len() >= 2 || (query.len() == 1 && query[0].len() >= 3)",
           ("Writfold.Core.Find.query_ok_spec",)),
    # find: every character of a prefix is compared, the last included
    Mutant("starts-skips-last", "find.rs",
           "    while i < start.len() {",
           "    while i + 1 < start.len() {",
           ("Writfold.Core.Find.starts_loop_spec",)),
    # find: the query token begins the name token, not the other way round
    Mutant("next-from-reversed", "find.rs",
           "        if starts(&name[j], q) {",
           "        if starts(q, &name[j]) {",
           ("Writfold.Core.Find.next_from_loop_spec",)),
    # find: each query token takes its own name token
    Mutant("prefixed-reuses-token", "find.rs",
           "            Some(j) => at = j + 1,",
           "            Some(j) => at = j,",
           ("Writfold.Core.Find.prefixed_loop_spec",)),
    # idle: the first of equal newest stamps is named
    Mutant("newest-last-of-equal", "idle.rs",
           "        Some((_, at)) => stamps[i] > at,",
           "        Some((_, at)) => stamps[i] >= at,",
           ("Writfold.Core.Idle.newer_spec",)),
    # idle: the age is compared over the integers, not after holding it in i64
    Mutant("idle-saturated-compare", "idle.rs",
           "            if crate::verdict::expired(now, last, age) {",
           "            if since(now, last) > age {",
           ("Writfold.Core.Idle.idle_spec",)),
    # idle: past the end of i64 the seconds shown are its end
    Mutant("since-off-by-one", "idle.rs",
           "    if last < 0 && now > i64::MAX + last {\n        return i64::MAX;",
           "    if last < 0 && now > i64::MAX + last {\n        return i64::MAX - 1;",
           ("Writfold.Core.Idle.since_spec",)),
    # assessed gate: one ungrounded premise makes the basis hypothetical
    Mutant("basis-ignores-premises", "assess.rs",
           "        if matches!(premises[index], Grounding::Ungrounded) {",
           "        if false {",
           ("Writfold.Core.Assess.basis_loop_spec",)),
    # assessed gate: a hypothetical entailment never meets the gate
    Mutant("met-accepts-hypothetical", "assess.rs",
           "    matches!(verdict, Verdict::Entailed) && matches!(basis, Basis::Grounded)",
           "    matches!(verdict, Verdict::Entailed) || matches!(basis, Basis::Grounded)",
           ("Writfold.Core.Assess.met_spec",)),
    # assessed gate: a check Weft could not finish is unknown, never a pass
    Mutant("outcome-passes-unfinished", "assess.rs",
           "        if unfinished(required[index]) {",
           "        if false {",
           ("Writfold.Core.Assess.outcome_loop_spec",)),
    # assessed gate: nothing required is unknown, never a pass
    Mutant("assessed-empty-passes", "assess.rs",
           "    if !complete || required.is_empty() {",
           "    if !complete {",
           ("Writfold.Core.Assess.outcome_spec",)),
    # assessed gate: an incomplete run is unknown, never a pass
    Mutant("assessed-incomplete-passes", "assess.rs",
           "    if !complete || required.is_empty() {",
           "    if required.is_empty() {",
           ("Writfold.Core.Assess.outcome_spec",)),
    # assessed gate: an attested citation makes the assumption attested
    Mutant("assumption-ignores-attested", "assess.rs",
           "        true => Grounding::Attested,",
           "        true => Grounding::Grounded,",
           ("Writfold.Core.Assess.assumption_loop_spec",)),
)

# lake reports `error: <path>.lean:<line>:<col>: <message>`
ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
DECLARATION = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)", re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)


def declarations(path: Path) -> list[tuple[int, str]]:
    """(line, fully qualified name) of every declaration in a theorem file."""
    namespace = ""
    found = []
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if match := NAMESPACE.match(line):
            namespace = match.group("name") + "."
            continue
        if line.startswith("end "):
            namespace = ""
            continue
        if match := DECLARATION.match(line):
            found.append((number, namespace + match.group("name")))
    return found


def attribute(file: Path, line: int) -> str | None:
    enclosing = None
    for start, name in declarations(file):
        if start <= line:
            enclosing = name
    return enclosing


def failing_theorems(output: str, project: Path) -> tuple[set[str], list[str]]:
    failures: set[str] = set()
    unattributed: list[str] = []
    for match in ERROR.finditer(output):
        file = (project / match.group("file")).resolve()
        try:
            relative = file.relative_to(project)
        except ValueError:
            unattributed.append(match.group(0))
            continue
        if relative.parts[0] != "Writfold" or not file.exists():
            unattributed.append(str(relative))
            continue
        name = attribute(file, int(match.group("line")))
        if name is None:
            unattributed.append(str(relative))
        else:
            failures.add(name)
    return failures, unattributed


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, timeout=1800)


def calibrate(mutant: Mutant, scratch: Path) -> None:
    crate = scratch / "crate"
    shutil.copytree(KERNEL, crate, ignore=shutil.ignore_patterns("target"))
    source = crate / "src" / mutant.file
    text = source.read_text()
    if text.count(mutant.before) != 1:
        raise SystemExit(f"FAIL {mutant.name}: mutation site does not match exactly once")
    source.write_text(text.replace(mutant.before, mutant.after))

    extraction = run(["sh", str(PROOFS / "extract.sh"), str(crate), str(scratch / "Extracted")], ROOT)
    if extraction.returncode != 0:
        print(extraction.stdout)
        raise SystemExit(f"FAIL {mutant.name}: the mutated kernel did not translate")

    project = scratch / "proofs"
    project.mkdir()
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Writfold.lean", "Audit.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(PROOFS / "Writfold", project / "Writfold")
    shutil.copytree(scratch / "Extracted", project / "Extracted")
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")

    build = run(["lake", "build"], project)
    if build.returncode == 0:
        raise SystemExit(f"FAIL {mutant.name}: the proofs still build against the mutated kernel")
    failures, unattributed = failing_theorems(build.stdout, project)
    if unattributed:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: errors outside the theorem files: {unattributed}")
    expected = set(mutant.fails)
    if not failures or not expected <= failures:
        print(build.stdout)
        print(f"attributed failures: {sorted(failures)}")
        raise SystemExit(f"FAIL {mutant.name}: expected {sorted(expected)} to fail, got {sorted(failures)}")
    if failures - expected:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: unexpected failing theorems {sorted(failures - expected)}")
    print(f"calibration: {mutant.name} rejected by {', '.join(sorted(failures))}", flush=True)


def unclaimed() -> list[str]:
    """Theorems a mutant names that the axiom audit does not require: a
    rename would then pass the audit and be caught only here."""
    claims = set(re.findall(r"``([\w.]+)", (PROOFS / "Audit.lean").read_text()))
    return sorted({name for m in MUTANTS for name in m.fails} - claims)


def main() -> None:
    if missing := unclaimed():
        raise SystemExit(f"FAIL: theorems named by mutants are not audited claims: {missing}")
    selected = set(sys.argv[1:])
    mutants = [m for m in MUTANTS if not selected or m.name in selected]
    if selected and len(mutants) != len(selected):
        raise SystemExit(f"unknown mutant(s): {sorted(selected - {m.name for m in mutants})}")
    # Every mutant runs even after a failure, so one gate run reports them all.
    failed: list[str] = []
    for mutant in mutants:
        with tempfile.TemporaryDirectory(prefix=f"mutant-{mutant.name}-") as directory:
            try:
                calibrate(mutant, Path(directory))
            except SystemExit as error:
                print(error, flush=True)
                failed.append(mutant.name)
    if failed:
        raise SystemExit(f"FAIL: {len(failed)} mutant(s) not calibrated: {failed}")
    print(f"PROOF_CALIBRATIONS={len(mutants)}", flush=True)


if __name__ == "__main__":
    main()
