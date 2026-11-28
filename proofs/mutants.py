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
import concurrent.futures
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
    # catalog (#84): a foreign slot is never written; a collision is a conflict
    Mutant("catalog-foreign-replaced", "catalog.rs",
           '        Owner::Foreign => {\n            if slot.wanted {\n                Action::Conflict\n            } else {\n                Action::Keep\n',
           '        Owner::Foreign => {\n            if slot.wanted {\n                Action::Replace\n            } else {\n                Action::Keep\n',
           ("Writfold.Core.Catalog.action_spec", "Writfold.Core.Catalog.replace_remove_only_owned", "Writfold.Core.Catalog.foreign_never_written",)),
    Mutant("catalog-foreign-removed", "catalog.rs",
           '        Owner::Foreign => {\n            if slot.wanted {\n                Action::Conflict\n            } else {\n                Action::Keep\n',
           '        Owner::Foreign => {\n            if slot.wanted {\n                Action::Conflict\n            } else {\n                Action::Remove\n',
           ("Writfold.Core.Catalog.action_spec", "Writfold.Core.Catalog.replace_remove_only_owned", "Writfold.Core.Catalog.foreign_never_written",)),
    # catalog: on its own slots the delta is exact
    Mutant("catalog-owned-keeps-unwanted", "catalog.rs",
           '            if !slot.wanted {\n                Action::Remove',
           '            if !slot.wanted {\n                Action::Keep',
           ("Writfold.Core.Catalog.action_spec", "Writfold.Core.Catalog.owned_delta_exact",)),
    Mutant("catalog-owned-rewrites-unchanged", "catalog.rs",
           '            } else if slot.equal {\n                Action::Keep',
           '            } else if slot.equal {\n                Action::Replace',
           ("Writfold.Core.Catalog.action_spec", "Writfold.Core.Catalog.owned_delta_exact",)),
    Mutant("catalog-creates-over-nothing-unwanted", "catalog.rs",
           '            if slot.wanted {\n                Action::Create\n            } else {\n                Action::Keep',
           '            if slot.wanted {\n                Action::Create\n            } else {\n                Action::Create',
           ("Writfold.Core.Catalog.action_spec", "Writfold.Core.Catalog.create_only_absent", "Writfold.Core.Catalog.absent_unwanted_kept",)),
    # catalog: partial input is never empty, a conflict never clean, drift never clean
    Mutant("catalog-partial-as-empty", "catalog.rs",
           '    if !complete || conflict {',
           '    if conflict {',
           ("Writfold.Core.Catalog.verdict_spec",)),
    Mutant("catalog-ignores-conflict", "catalog.rs",
           '    if !complete || conflict {',
           '    if !complete {',
           ("Writfold.Core.Catalog.verdict_spec",)),
    Mutant("catalog-dirty-as-clean", "catalog.rs",
           '    if changed {\n        return Verdict::Drift;',
           '    if false {\n        return Verdict::Drift;',
           ("Writfold.Core.Catalog.verdict_spec",)),
    # catalog: the folds see every slot; an invalid plan emits no patch
    Mutant("catalog-conflict-skips-last", "catalog.rs",
           '    while index < slots.len() {\n        if conflicts(',
           '    while index + 1 < slots.len() {\n        if conflicts(',
           ("Writfold.Core.Catalog.any_conflict_loop_spec",)),
    Mutant("catalog-change-skips-last", "catalog.rs",
           '    while index < slots.len() {\n        if changes(',
           '    while index + 1 < slots.len() {\n        if changes(',
           ("Writfold.Core.Catalog.any_change_loop_spec",)),
    Mutant("catalog-invalid-patches", "catalog.rs",
           '        Verdict::Invalid => false,',
           '        Verdict::Invalid => true,',
           ("Writfold.Core.Catalog.patches_spec", "Writfold.Core.Catalog.partial_never_patches",)),
    # handoff (#67): unread evidence is never accepted
    Mutant("checked-ignores-evidence", "handoff.rs",
           "    if !evidence_complete {",
           "    if false {",
           ("Writfold.Core.Handoff.incomplete_evidence_never_accepted", "Writfold.Core.Handoff.accepted_implies_evidence_complete")),
    # step.settled (#79): only a spent loop settles an owing step
    Mutant("settled-ignores-spent", "guard/require.rs",
           "            if spent {",
           "            if false {",
           ("Writfold.Core.Guard.settled_met_iff",)),
    Mutant("settled-always", "guard/require.rs",
           "            if spent {",
           "            if true {",
           ("Writfold.Core.Guard.settled_met_iff",)),
    # step.settled: an unread plan is never settled
    Mutant("settled-ignores-read", "guard/require.rs",
           "        Looked::Failed => Fact::Unknown,\n        Looked::Read => {\n            if spent {",
           "        Looked::Failed => Fact::Met,\n        Looked::Read => {\n            if spent {",
           ("Writfold.Core.Guard.settled_met_iff", "Writfold.Core.Guard.settled_unknown_iff_unread")),
    # loops (#75): the round kernel
    Mutant("round-exit-ignores-min", "plan/round.rs",
           "    if rounds >= b.min && all_met(met) {",
           "    if all_met(met) {",
           ("Writfold.Core.Round.exit_spec",)),
    Mutant("round-again-ignores-max", "plan/round.rs",
           "    if rounds < b.max {\n        return Exit::Again;",
           "    if true {\n        return Exit::Again;",
           ("Writfold.Core.Round.exit_spec",)),
    Mutant("round-again-when-met", "plan/round.rs",
           "        Exit::Met => Err(AgainRefusal::Met),",
           "        Exit::Met => Ok(rounds + 1),",
           ("Writfold.Core.Round.again_spec",)),
    Mutant("round-again-any-step", "plan/round.rs",
           "    if !looped {",
           "    if false {",
           ("Writfold.Core.Round.again_spec",)),
    Mutant("round-advance-skips-gates", "plan/round.rs",
           "        Err(refusal) => Err(Refusal::Plan(refusal)),",
           "        Err(_refusal) => Ok(()),",
           ("Writfold.Core.Round.advance_iff",)),
    Mutant("round-advance-ignores-min", "plan/round.rs",
           "            if rounds < b.min {",
           "            if false {",
           ("Writfold.Core.Round.advance_iff",)),
    Mutant("round-superseded-newest", "plan/round.rs",
           "    place.round < place.rounds",
           "    place.round <= place.rounds",
           ("Writfold.Core.Round.superseded_iff",)),
    Mutant("round-superseded-any-skips-last", "plan/round.rs",
           "    while index < path.len() {",
           "    while index + 1 < path.len() {",
           ("Writfold.Core.Round.superseded_any_loop_spec",)),
    Mutant("round-stale-ignores-superseded", "plan/round.rs",
           "    if superseded {\n        return false;",
           "    if false {\n        return false;",
           ("Writfold.Core.Round.superseded_never_stale",)),
    Mutant("round-over-at-max", "plan/round.rs",
           "    rounds > b.max",
           "    rounds >= b.max",
           ("Writfold.Core.Round.over_iff",)),
    # round: a gate counts as judged only when its verdict says so
    Mutant("round-judged-at-ignores-verdict", "plan/round.rs",
           "    i < judged.len() && judged[i]\n",
           "    i < judged.len()\n",
           ("Writfold.Core.Round.judged_at_spec",)),
    Mutant("round-next-again-before-judge", "plan/round.rs",
           "            if judged_at(judged, i) {",
           "            if true {",
           ("Writfold.Core.Round.owed_or_spec",)),
    # walk (#60): hitting the work limit never certifies completeness
    Mutant("within-ignores-truncation", "walk.rs",
           "    if truncated {\n        return false;\n    }\n    complete(frontier, skipped)",
           "    if truncated && false {\n        return false;\n    }\n    complete(frontier, skipped)",
           ("Writfold.Core.Walk.complete_within_iff", "Writfold.Core.Walk.truncated_never_complete")),
    # graph (#59): every file's change is seen
    Mutant("any-skips-last", "graph.rs",
           "    while index < changed.len() {",
           "    while index + 1 < changed.len() {",
           ("Writfold.Core.Graph.any_loop_spec",)),
    # graph (#65): an affected file is never skipped
    Mutant("plan-skip-ignores-changes", "graph.rs",
           "    if any(affected) {",
           "    if any(affected) && false {",
           ("Writfold.Core.Graph.plan_skip_iff", "Writfold.Core.Graph.plan_delta_iff")),
    # graph (#65): each cause makes a file affected
    Mutant("affected-ignores-dups", "graph.rs",
           "    refs_changed || dup_changed || token_moved",
           "    refs_changed || token_moved",
           ("Writfold.Core.Graph.affected_iff",)),
    Mutant("affected-ignores-tokens", "graph.rs",
           "    refs_changed || dup_changed || token_moved",
           "    refs_changed || dup_changed",
           ("Writfold.Core.Graph.affected_iff",)),
    # publish (#58): drift never authorises a repair
    Mutant("repair-on-drift", "publish.rs",
           "    if found.source_moved || found.live_moved {",
           "    if false {",
           ("Writfold.Core.Publish.repair_replace_iff", "Writfold.Core.Publish.drift_never_replaces")),
    # publish: an equal index is kept
    Mutant("repair-without-mismatch", "publish.rs",
           "    if !found.mismatch {",
           "    if false {",
           ("Writfold.Core.Publish.repair_replace_iff", "Writfold.Core.Publish.equal_keeps")),
    # publish: a rebuild read while the KB moved is not published
    Mutant("derived-ignores-move", "publish.rs",
           "    if source_moved {",
           "    if false {",
           ("Writfold.Core.Publish.derived_replace_iff",)),
    # packet (#109): the cut never overruns the budget
    Mutant("cut-ignores-used", "packet.rs",
           "        if cost > budget - used {",
           "        if cost > budget {",
           ("Writfold.Core.Packet.cut_loop_spec",)),
    # packet: a row that fits exactly is kept
    Mutant("cut-drops-exact-fit", "packet.rs",
           "        if cost > budget - used {",
           "        if cost >= budget - used {",
           ("Writfold.Core.Packet.cut_loop_spec",)),
    # packet: an unreadable input makes the packet partial
    Mutant("packet-complete-ignores-unreadable", "packet.rs",
           "    kept >= required && unreadable == 0 && unread == 0",
           "    kept >= required && unread == 0",
           ("Writfold.Core.Packet.complete_iff",)),
    # account (#181): every left-over member is counted
    Mutant("remainder-skips-last", "account.rs",
           "    while index < members.len() {\n        if left(members[index]) {",
           "    while index + 1 < members.len() {\n        if left(members[index]) {",
           ("Writfold.Core.Account.remainder_loop_spec",)),
    Mutant("declared-out-counts", "account.rs",
           "        Member::Remainder => true,\n        Member::Judged | Member::DeclaredOut => false,",
           "        Member::Remainder | Member::DeclaredOut => true,\n        Member::Judged => false,",
           ("Writfold.Core.Account.left_spec",)),
    # packet (#181): a file left unread keeps a packet partial
    Mutant("complete-ignores-unread", "packet.rs",
           "    kept >= required && unreadable == 0 && unread == 0",
           "    kept >= required && unreadable == 0",
           ("Writfold.Core.Packet.complete_iff",)),
    # packet: the left-out required rows are counted
    Mutant("omitted-reads-zero", "packet.rs",
           "    if kept >= required { 0 } else { required - kept }",
           "    if kept >= required { 0 } else { 0 }",
           ("Writfold.Core.Packet.required_omitted_spec",)),
    # prose (#127): an empty rule set is never a pass
    Mutant("prose-no-rules-pass", "prose.rs",
           "    if rules == 0 {\n        return Outcome::Unknown;\n    }",
           "    if rules == 0 {\n        return Outcome::Pass;\n    }",
           ("Writfold.Core.Prose.verdict_spec",)),
    # prose: advice is never counted as blocking
    Mutant("prose-advice-blocks", "prose.rs",
           "            Weight::Advice => {}",
           "            Weight::Advice => count += 1,",
           ("Writfold.Core.Prose.blocking_loop_spec",)),
    # prose: a blocking match fails the check
    Mutant("prose-blocking-ignored", "prose.rs",
           "    if blocking(found) > 0 {",
           "    if blocking(found) > usize::MAX {",
           ("Writfold.Core.Prose.verdict_spec",)),
    # prose: a pass agrees only with no blocking findings
    Mutant("prose-agrees-pass-any", "prose.rs",
           "        Outcome::Pass => blocking == 0,",
           "        Outcome::Pass => true,",
           ("Writfold.Core.Prose.agrees_spec",)),
    # compose (#144): the last required tool is checked too
    Mutant("covers-ignores-last", "compose.rs",
           "    while index < required.len() {",
           "    while index + 1 < required.len() {",
           ("Writfold.Core.Compose.covers_loop_spec",)),
    # compose: a definition of the wrong kind never resolves a reference
    Mutant("closure-wrong-kind-found", "compose.rs",
           "            RefFact::Found => {}\n            RefFact::Missing | RefFact::WrongKind | RefFact::Twice => {",
           "            RefFact::Found | RefFact::WrongKind => {}\n            RefFact::Missing | RefFact::Twice => {",
           ("Writfold.Core.Compose.closure_loop_spec",)),
    # compose: a skill's tools must be granted by the profile
    Mutant("closure-skips-tools", "compose.rs",
           "        Some(tool) => Err(Refusal::Tool(tool)),",
           "        Some(_) => Ok(()),",
           ("Writfold.Core.Compose.closure_loop_spec",)),
    # invoke (#148): a required packet that is not complete is refused
    Mutant("prepare-skips-packet", "invoke.rs",
           "    if facts.packet_required && !facts.packet_complete {\n        return Err(PrepareRefusal::PacketIncomplete);\n    }\n",
           "",
           ("Writfold.Core.Invoke.prepare_check_spec",)),
    # invoke (#147): a skill's plan template must be in the run
    Mutant("prepare-skips-plan", "invoke.rs",
           "    if !facts.plans_in_run {\n        return Err(PrepareRefusal::PlanNotInRun);\n    }\n",
           "",
           ("Writfold.Core.Invoke.prepare_check_spec",)),
    # invoke (#149): the closure as it is now must be the one prepared
    Mutant("bound-ignores-closure", "invoke.rs",
           "        && named.closure_current\n",
           "\n",
           ("Writfold.Core.Invoke.bound_spec",)),
    # invoke (#149): the invocation's step must be the step judged
    Mutant("bound-ignores-step", "invoke.rs",
           "        && named.this_step\n",
           "\n",
           ("Writfold.Core.Invoke.bound_spec",)),
    # invoke (#149): a second report for the invocation fails both
    Mutant("bound-ignores-single", "invoke.rs",
           "        && named.single\n",
           "\n",
           ("Writfold.Core.Invoke.bound_spec",)),
    # invoke (#149): an unreadable record is never a pass
    Mutant("bound-unread-passes", "invoke.rs",
           "        return Outcome::Unknown;",
           "        return Outcome::Pass;",
           ("Writfold.Core.Invoke.bound_spec",)),
    # invoke (#154): a need not met refuses prepare
    Mutant("prepare-skips-needs", "invoke.rs",
           "            Some(index) => Err(PrepareRefusal::Needs(index)),",
           "            Some(_) => Ok(()),",
           ("Writfold.Core.Invoke.prepare_spec",)),
    # invoke (#154): a need that could not be judged is owed
    Mutant("needs-unknown-met", "invoke.rs",
           "            Fact::Met => {}\n            Fact::Unmet | Fact::Unknown => return Some(index),",
           "            Fact::Met | Fact::Unknown => {}\n            Fact::Unmet => return Some(index),",
           ("Writfold.Core.Invoke.first_unmet_loop_spec",)),
    # units (#156): the last member pairs too
    Mutant("pairs-drops-last", "units.rs",
           "        while b < n {",
           "        while b + 1 < n {",
           ("Writfold.Core.Units.pairs_inner_spec",)),
    # units (#156): a member never pairs with itself
    Mutant("pairs-keeps-diagonal", "units.rs",
           "        let mut b = a + 1;",
           "        let mut b = a;",
           ("Writfold.Core.Units.pairs_outer_spec",)),
    # units (#156): an absent universe is never covered
    Mutant("generated-absent-covered", "units.rs",
           "    let universe = present && (count > 0 || allow_empty);",
           "    let universe = count > 0 || allow_empty;",
           ("Writfold.Core.Units.generated_covered_iff",)),
    # units (#156): an empty universe needs allow_empty
    Mutant("generated-empty-covered", "units.rs",
           "    let universe = present && (count > 0 || allow_empty);",
           "    let universe = present;",
           ("Writfold.Core.Units.generated_covered_iff",)),
    # ground (#156): an unready report grounds nothing
    Mutant("delivery-unready-grounds", "ground.rs",
           "    if found && live && ready && bound {",
           "    if found && live && bound {",
           ("Writfold.Core.Ground.of_delivery_grounded_iff",)),
    # ground (#164): a withdrawn report grounds nothing
    Mutant("delivery-withdrawn-grounds", "ground.rs",
           "    if found && live && ready && bound {",
           "    if found && ready && bound {",
           ("Writfold.Core.Ground.of_delivery_grounded_iff",
            "Writfold.Core.Ground.withdrawn_never_grounds")),
    # invoke (#167): --out inside the KB is refused
    Mutant("prepare-skips-out", "invoke.rs",
           "    if !facts.out_outside {\n        return Err(PrepareRefusal::OutInsideKb);\n    }\n",
           "",
           ("Writfold.Core.Invoke.prepare_check_spec",)),
    # invoke (#167): --for an unknown item is refused
    Mutant("prepare-skips-for", "invoke.rs",
           "        ForItem::Unknown => Err(PrepareRefusal::ForUnknown),",
           "        ForItem::Unknown => Ok(()),",
           ("Writfold.Core.Invoke.prepare_check_spec",)),
    # units (#165): an attested scope gate gives no universe
    Mutant("universe-attested-scope", "units.rs",
           "        Grounding::Grounded => {}\n        Grounding::Attested | Grounding::Ungrounded => return Err(Absent::NotChecked),",
           "        Grounding::Grounded | Grounding::Attested => {}\n        Grounding::Ungrounded => return Err(Absent::NotChecked),",
           ("Writfold.Core.Units.universe_first",)),
    # units (#165): a partial manifest gives no universe
    Mutant("universe-partial-present", "units.rs",
           "    if !facts.complete {\n        return Err(Absent::Partial);\n    }\n",
           "",
           ("Writfold.Core.Units.universe_first",)),
    # units (#165): a member outside the scope gives no universe
    Mutant("universe-outside-scope-present", "units.rs",
           "    if !facts.inside {\n        return Err(Absent::OutsideScope);\n    }\n",
           "",
           ("Writfold.Core.Units.universe_first",)),
    # units (#165): over the cap gives no universe
    Mutant("universe-over-cap-present", "units.rs",
           "    if facts.members > MAX_MEMBERS {\n        return Err(Absent::OverCap);\n    }\n",
           "",
           ("Writfold.Core.Units.universe_first",)),
    # units (#166): each of a unit's inputs is consulted
    Mutant("unit-ignores-second", "units.rs",
           "    if !second_same {\n        return Some(Moved::Second);\n    }\n",
           "",
           ("Writfold.Core.Units.unit_first_moved",
            "Writfold.Core.Units.unit_current_iff")),
    Mutant("unit-ignores-seed", "units.rs",
           "    if !seed_same {\n        return Some(Moved::Seed);\n    }\n",
           "",
           ("Writfold.Core.Units.unit_first_moved",
            "Writfold.Core.Units.unit_current_iff")),
    Mutant("unit-ignores-definition", "units.rs",
           "    if !definition_same {\n        return Some(Moved::Definition);\n    }\n",
           "",
           ("Writfold.Core.Units.unit_first_moved",
            "Writfold.Core.Units.unit_current_iff")),
    # target (#179): the resolver never guesses
    Mutant("pin-missing-falls-back", "target.rs",
           "        Pinned::Missing => return Choice::Ambiguous(Why::PinMissing),",
           "        Pinned::Missing => {}",
           ("Writfold.Core.Target.choose_in_spec", "Writfold.Core.Target.pin_decides", "Writfold.Core.Target.pin_decides_in")),
    Mutant("nohead-takes-head", "target.rs",
           "    if headless_at(remotes, level, declared) {\n        return Choice::Ambiguous(Why::NoDefaultBranch);\n    }\n",
           "",
           ("Writfold.Core.Target.choose_in_spec",)),
    Mutant("disagree-takes-first", "target.rs",
           "            false => Choice::Ambiguous(Why::Disagree),",
           "            false => Choice::Remote(i),",
           ("Writfold.Core.Target.choose_in_spec",)),
    Mutant("first-remote-wins", "target.rs",
           "        let r = remotes[i];\n        if rank(r.names, declared) == level {\n            return match",
           "        let r = remotes[i];\n        if rank(r.names, declared) == level || i == 0 {\n            return match",
           ("Writfold.Core.Target.first_at_loop_spec",)),
    Mutant("best-skips-last", "target.rs",
           "    while i < remotes.len() {\n        let level = rank(remotes[i].names, declared);",
           "    while i + 1 < remotes.len() {\n        let level = rank(remotes[i].names, declared);",
           ("Writfold.Core.Target.best_loop_spec",)),
    Mutant("headless-ignores-level", "target.rs",
           "        if rank(r.names, declared) == level && tip_commit(r.tip).is_none() {",
           "        if tip_commit(r.tip).is_none() {",
           ("Writfold.Core.Target.headless_at_loop_spec", "Writfold.Core.Target.headless_at_spec")),
    Mutant("agree-ignores-level", "target.rs",
           "        if rank(r.names, declared) == level && !tip_is(r.tip, k) {",
           "        if !tip_is(r.tip, k) {",
           ("Writfold.Core.Target.agree_at_loop_spec", "Writfold.Core.Target.agree_at_spec")),
    # target (#197): a declared repository is only its declared URL
    Mutant("declared-admits-path", "target.rs",
           "        (Names::Path, false) => 3,",
           "        (Names::Path, _) => 3,",
           ("Writfold.Core.Target.rank_spec",)),
    Mutant("declared-takes-head", "target.rs",
           "            true => Choice::Ambiguous(Why::NoRemoteNames),\n            false => Choice::Head,",
           "            true => Choice::Head,\n            false => Choice::Head,",
           ("Writfold.Core.Target.choose_in_spec",)),
    Mutant("declared-ignores-pin", "target.rs",
           "        Pinned::Missing => return Choice::Ambiguous(Why::PinMissing),",
           "        Pinned::Missing if !declared => return Choice::Ambiguous(Why::PinMissing),\n        Pinned::Missing => {}",
           ("Writfold.Core.Target.choose_in_spec", "Writfold.Core.Target.pin_decides_in")),
    # verdict (#182): bytes a partial clone lacks decide nothing
    Mutant("unfetched-changed", "verdict.rs",
           "        Some(Located::Unfetched) => Verdict::Unverifiable,\n        _ => {",
           "        Some(Located::Unfetched) => Verdict::Changed,\n        _ => {",
           ("Writfold.Core.Verdict.other_blob_spec",)),
    Mutant("unfetched-fresh", "verdict.rs",
           "        Some(Located::Unfetched) => Verdict::Unverifiable,\n        _ => {",
           "        Some(Located::Unfetched) => Verdict::Fresh,\n        _ => {",
           ("Writfold.Core.Verdict.other_blob_spec",)),
    Mutant("unfetched-gone", "verdict.rs",
           "        Absence::Unfetched => Verdict::Unverifiable,",
           "        Absence::Unfetched => Verdict::Gone,",
           ("Writfold.Core.Verdict.missing_spec",)),
    # referent (#196): the boundary only withholds
    Mutant("ambiguous-judged", "referent.rs",
           "    if !resolved {\n        return Verdict::Unverifiable;\n    }\n",
           "",
           ("Writfold.Core.Referent.settle_spec",)),
    Mutant("behind-keeps-gone", "referent.rs",
           "        Verdict::Changed | Verdict::Gone | Verdict::Moved | Verdict::Renamed",
           "        Verdict::Changed | Verdict::Moved | Verdict::Renamed",
           ("Writfold.Core.Referent.settle_spec",)),
    Mutant("behind-drops-fresh", "referent.rs",
           "        Verdict::Changed | Verdict::Gone | Verdict::Moved | Verdict::Renamed",
           "        Verdict::Fresh | Verdict::Changed | Verdict::Gone | Verdict::Moved | Verdict::Renamed",
           ("Writfold.Core.Referent.settle_spec",)),
    Mutant("judge-skips-settle", "referent.rs",
           "    settle(resolved, relation, observed(fact))",
           "    let _ = (resolved, relation);\n    observed(fact)",
           ("Writfold.Core.Referent.judge_spec",)),
    # vale (#131): only the pinned executable's run is read
    Mutant("vale-absent-usable", "vale.rs",
           "        Tool::Absent | Tool::Other => return false,",
           "        Tool::Absent => {}\n        Tool::Other => return false,",
           ("Writfold.Core.Vale.usable_spec",)),
    Mutant("vale-other-version-usable", "vale.rs",
           "        Tool::Absent | Tool::Other => return false,",
           "        Tool::Other => {}\n        Tool::Absent => return false,",
           ("Writfold.Core.Vale.usable_spec",)),
    # vale: exit 2 (a runtime or config error) is never read
    Mutant("vale-exit2-usable", "vale.rs",
           "        Ending::Exited(_) | Ending::Signaled | Ending::TimedOut | Ending::Error => false,",
           "        Ending::Exited(_) => true,\n        Ending::Signaled | Ending::TimedOut | Ending::Error => false,",
           ("Writfold.Core.Vale.usable_spec",)),
    # vale: exit 0 must agree with no error-level alert
    Mutant("vale-exit-disagrees-usable", "vale.rs",
           "        Ending::Exited(0) => ran.errors == 0,",
           "        Ending::Exited(0) => true,",
           ("Writfold.Core.Vale.usable_spec",)),
    # vale: cut-off or unreadable output is never read
    Mutant("vale-cut-usable", "vale.rs",
           "    if !ran.whole {\n        return false;\n    }",
           "    if !ran.whole {\n        return true;\n    }",
           ("Writfold.Core.Vale.usable_spec",)),
    # vale: a silent canary means no rule applied
    Mutant("vale-no-canary-pass", "vale.rs",
           "    if canary == 0 { 0 } else { enabled }",
           "    if canary == 0 { enabled } else { enabled }",
           ("Writfold.Core.Vale.rules_spec",)),
    # vale: with vale_block = "never" nothing blocks
    Mutant("vale-never-blocks", "vale.rs",
           "        BlockFrom::Never => Weight::Advice,",
           "        BlockFrom::Never => Weight::Blocking,",
           ("Writfold.Core.Vale.weigh_spec",)),
    # vale: with vale_block = "warning" a warning blocks
    Mutant("vale-warning-skips", "vale.rs",
           "            Level::Error | Level::Warning => Weight::Blocking,\n            Level::Suggestion => Weight::Advice,",
           "            Level::Error => Weight::Blocking,\n            Level::Warning | Level::Suggestion => Weight::Advice,",
           ("Writfold.Core.Vale.weigh_spec",)),
    # vale: an unreadable run is unknown, never a failure
    Mutant("vale-unusable-fail", "vale.rs",
           "        return Outcome::Unknown;",
           "        return Outcome::Fail;",
           ("Writfold.Core.Vale.settle_spec",)),
    # runs (#98): --open never hides a state outside the vocabulary
    Mutant("other-hidden-by-open", "runs.rs",
           "        State::Other => Shown::Flagged,",
           "        State::Other => {\n            if open_only {\n                Shown::Hidden\n            } else {\n                Shown::Flagged\n            }\n        }",
           ("Writfold.Core.Runs.shown_spec",)),
    # runs: an unrecognized state is flagged, never listed as ordinary
    Mutant("other-listed-plain", "runs.rs",
           "        State::Other => Shown::Flagged,",
           "        State::Other => Shown::Listed,",
           ("Writfold.Core.Runs.shown_spec",)),
    # runs: a run directory without a manifest makes the list partial
    Mutant("complete-ignores-no-manifest", "runs.rs",
           "    unreadable == 0 && no_manifest == 0",
           "    unreadable == 0",
           ("Writfold.Core.Runs.complete_spec",)),
    # inventory (#56): an unreadable root is never readable
    Mutant("readable-all", "inventory.rs",
           "    !matches!(root, Root::Unreadable)",
           "    true",
           ("Writfold.Core.Inventory.readable_spec",)),
    # inventory: an unreadable root makes it incomplete
    Mutant("complete-skips-check", "inventory.rs",
           "        if !readable(&roots[index]) {\n            return false;",
           "        if false {\n            return false;",
           ("Writfold.Core.Inventory.complete_loop_spec",)),
    # inventory: each unreadable root counts as one problem
    Mutant("unreadable-never-counts", "inventory.rs",
           "            count += 1;",
           "            count += 0;",
           ("Writfold.Core.Inventory.unreadable_loop_spec",)),
    # inventory: an incomplete inventory never gives clean planned lines
    Mutant("over-drops-complete", "inventory.rs",
           "    complete && lines_clean(summary, listed, valid)",
           "    lines_clean(summary, listed, valid)",
           ("Writfold.Core.Inventory.lines_clean_over_iff", "Writfold.Core.Inventory.lines_clean_over_incomplete")),
    # inventory: every judged gate must be met
    Mutant("all-true-skips-last", "inventory.rs",
           "    while index < met.len() {",
           "    while index + 1 < met.len() {",
           ("Writfold.Core.Inventory.all_true_loop_spec",)),
    # inventory: no gate judged is never clean
    Mutant("unplanned-empty-clean", "inventory.rs",
           "    complete && !met.is_empty() && all_true(met) && valid",
           "    complete && all_true(met) && valid",
           ("Writfold.Core.Inventory.unplanned_clean_iff",)),
    # handoff (#57): evidence under another definition is never a pass
    Mutant("item-ignores-approved", "handoff.rs",
           "    if !item.approved {",
           "    if false {",
           ("Writfold.Core.Handoff.item_spec",)),
    # handoff: every required gate is examined
    Mutant("first-failing-skips-last", "handoff.rs",
           "    while at < items.len() {",
           "    while at + 1 < items.len() {",
           ("Writfold.Core.Handoff.first_failing_loop_spec",)),
    # handoff: an empty required set is refused
    Mutant("strict-empty-ok", "handoff.rs",
           "    if items.is_empty() {",
           "    if false {",
           ("Writfold.Core.Handoff.strict_spec",)),
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
    # gate (#121): missing retained evidence never passes
    Mutant("kept-missing-passes", "gate.rs",
           "            Outcome::Pass => Outcome::Unknown,",
           "            Outcome::Pass => Outcome::Pass,",
           ("Writfold.Core.Gate.outcome_kept_spec",)),
    # gate: keeping every file changes nothing
    Mutant("kept-all-unknown", "gate.rs",
           "        Kept::Nothing | Kept::All => o,",
           "        Kept::Nothing => o,\n        Kept::All => Outcome::Unknown,",
           ("Writfold.Core.Gate.outcome_kept_spec",)),
    # gate: missing evidence leaves a failure a failure
    Mutant("kept-fail-unknown", "gate.rs",
           "            Outcome::Fail => Outcome::Fail,",
           "            Outcome::Fail => Outcome::Unknown,",
           ("Writfold.Core.Gate.outcome_kept_spec",)),
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
           "    entry.required && !covered_under(entry.item, entry.na, entry.none, entry.unsnapped)",
           "    !covered_under(entry.item, entry.na, entry.none, entry.unsnapped)",
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
           "    if owed.is_empty() {\n        return Standing::Checked(State::Open);",
           "    if owed.is_empty() {\n        return Standing::Checked(State::Satisfied);",
           ("Writfold.Core.Delivery.standing_spec",)),
    # delivery: a waived obligation is never read as a passing check
    Mutant("delivery-waived-satisfied", "delivery.rs",
           "    if waived {\n        Standing::Waived",
           "    if false {\n        Standing::Waived",
           ("Writfold.Core.Delivery.standing_loop_spec",)),
    # delivery: an unmet obligation stops the delivery
    Mutant("delivery-unmet-ignored", "delivery.rs",
           "    !crate::policy::met(owed.standing, &owed.accept)",
           "    false",
           ("Writfold.Core.Delivery.unmet_spec",)),
    # delivery: a bound candidate ends the scan (continuing past it is the
    # rule before 17 November 2026, under which an abandoned draft met a step)
    Mutant("deciding-ignores-bound", "delivery.rs",
           "        if is_bound(&places[index]) {\n            return None;",
           "        if false {\n            return None;",
           ("Writfold.Core.Delivery.deciding_loop_spec",)),
    # delivery: the scan runs from the newest candidate, not the oldest
    Mutant("deciding-first-unbound", "delivery.rs",
           "    let mut index = places.len();\n    while index > 0 {\n        index -= 1;\n        if is_open(&places[index]) {\n            return Some(index);",
           "    let mut index = 0;\n    while index < places.len() {\n        index += 1;\n        if is_open(&places[index - 1]) {\n            return Some(index - 1);",
           ("Writfold.Core.Delivery.deciding_loop_spec", "Writfold.Core.Delivery.deciding_spec")),
    # delivery: a mode that declares nothing is never ready
    Mutant("ready-accepts-empty", "delivery.rs",
           "    if owed.is_empty() {\n        return false;",
           "    if owed.is_empty() {\n        return true;",
           ("Writfold.Core.Delivery.ready_iff",)),
    # delivery: a passed check whose cited evidence moved is stale
    Mutant("rechecked-keeps-satisfied", "delivery.rs",
           "    if matches!(checked, State::Satisfied) && moved > 0 {",
           "    if false {",
           ("Writfold.Core.Delivery.rechecked_spec",)),
    # delivery guard: unread bytes under a listed command need a live candidate
    Mutant("guard-allows-no-candidate", "delivery.rs",
           "        index += 1;\n    }\n    live\n}",
           "        index += 1;\n    }\n    true\n}",
           ("Writfold.Core.Delivery.guard_all_loop_spec",)),
    # delivery guard: known bytes pass only when a postable candidate holds them
    Mutant("guard-known-any-ready", "delivery.rs",
           "        Body::Known => guard_any(seen),",
           "        Body::Known => guard_any(seen) || guard_all(seen),",
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
           "owed: usize) -> Fact {\n    match read {\n        Looked::No => Fact::Unmet,\n        Looked::Failed => Fact::Unknown,",
           "owed: usize) -> Fact {\n    match read {\n        Looked::No => Fact::Unmet,\n        Looked::Failed => Fact::Met,",
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
           # `units::universe` grounds its scope gate through `of_gate` (#165).
           ("Writfold.Core.Ground.of_gate_spec",
            "Writfold.Core.Units.universe_first")),
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
           "    if c.starred && c.at < pattern.len() && c.from < text.len() {",
           "    if false && c.at < pattern.len() && c.from < text.len() {",
           ("Writfold.Core.Scope.char_step_spec",)),
    # scope: `*` matches nothing too, at the end of a segment included
    Mutant("star-needs-a-char", "scope.rs",
           "        && *p == STAR\n",
           "        && *p == STAR\n        && c.j < text.len()\n",
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
           "        && globstar(w)\n",
           "        && globstar(w)\n        && c.j < parts.len()\n",
           ("Writfold.Core.Scope.path_step_spec",)),
    # scope: a path with segments left over after the pattern does not match
    Mutant("segments-accept-trailing", "scope.rs",
           "    let part = match parts.get(c.j) {",
           "    if let None = here {\n        return Next::Done(c.i == wanted.len());\n    }\n"
           "    let part = match parts.get(c.j) {",
           ("Writfold.Core.Scope.path_step_spec", "Writfold.Core.Scope.path_step_outside")),
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
    # verdict: a same blob is fresh (the table itself)
    Mutant("same-blob-changed", "verdict.rs",
           "        Observed::Code(Target::SameBlob) => Verdict::Fresh,",
           "        Observed::Code(Target::SameBlob) => Verdict::Changed,",
           ("Writfold.Core.Verdict.observed_spec",)),
    # verdict: a span found at other lines is moved, never fresh
    Mutant("moved-span-fresh", "verdict.rs",
           "        Some(Located::At(_, _)) => Verdict::Moved,\n        Some(Located::Absent) => Verdict::Changed,",
           "        Some(Located::At(_, _)) => Verdict::Fresh,\n        Some(Located::Absent) => Verdict::Changed,",
           ("Writfold.Core.Verdict.other_blob_spec",)),
    # verdict: a file too long to search is changed, never fresh
    Mutant("unsearched-fresh", "verdict.rs",
           "            if known {\n                Verdict::Changed",
           "            if known {\n                Verdict::Fresh",
           ("Writfold.Core.Verdict.other_blob_spec",)),
    # verdict (#90): with no cited blob known and no span, there is no evidence of a change
    Mutant("no-blob-no-span-changed", "verdict.rs",
           "            } else {\n                Verdict::Unverifiable\n",
           "            } else {\n                Verdict::Changed\n",
           ("Writfold.Core.Verdict.other_blob_spec",)),
    # verdict (#90): a cited commit absent from this clone is not a removal
    Mutant("rev-absent-gone", "verdict.rs",
           "        Absence::RevAbsent => Verdict::Unverifiable,",
           "        Absence::RevAbsent => Verdict::Gone,",
           ("Writfold.Core.Verdict.missing_spec",)),
    # verdict (#90): a work limit is never reported as a removal
    Mutant("capped-gone", "verdict.rs",
           "        Absence::Capped => Verdict::Unverifiable,",
           "        Absence::Capped => Verdict::Gone,",
           ("Writfold.Core.Verdict.missing_spec",)),
    # verdict (#90): a followed rename is never fresh
    Mutant("followed-span-fresh", "verdict.rs",
           "            Some(Located::Same) => Verdict::Moved,",
           "            Some(Located::Same) => Verdict::Fresh,",
           ("Writfold.Core.Verdict.missing_spec",)),
    # verdict: a mark never hides a change
    Mutant("mark-hides-change", "verdict.rs",
           "        Verdict::Changed | Verdict::Gone | Verdict::Expired => reached,",
           "        Verdict::Gone | Verdict::Expired => reached,",
           ("Writfold.Core.Verdict.with_mark_spec",)),
    # verdict: decide applies the authored mark (#212)
    Mutant("decide-drops-mark", "verdict.rs",
           "    with_mark(observed(fact), marked)",
           "    with_mark(observed(fact), false)",
           ("Writfold.Core.Verdict.decide_spec",)),
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
    # check receipts: only a told statement may decide (#212)
    Mutant("usable-unverifiable", "verdict/recorded.rs",
           "    !matches!(s.said, Said::Unverifiable)",
           "    !matches!(s.said, Said::Gone)",
           ("Writfold.Core.Recorded.usable_spec",)),
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
           "                if before(candidate, kept) {",
           "                if !before(kept, candidate) {",
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
           "            Some((_, at)) => *stamp > at,",
           "            Some((_, at)) => *stamp >= at,",
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
    # order: at the first differing byte, the smaller sorts first
    Mutant("less-larger-byte", "order.rs",
           "            return a[i] < b[i];",
           "            return a[i] > b[i];",
           ("Writfold.Core.Order.less_loop_spec",)),
    # order: a name never sorts before itself
    Mutant("less-equal-names", "order.rs",
           "    a.len() < b.len()\n}",
           "    a.len() <= b.len()\n}",
           ("Writfold.Core.Order.less_loop_spec",)),
    # order: the lower seq is judged first
    Mutant("precedes-higher-seq", "order.rs",
           "        return seq_a < seq_b;",
           "        return seq_a > seq_b;",
           ("Writfold.Core.Order.precedes_spec",)),
    # store: a receipt stands in only under the pin (#4)
    Mutant("recorded-any-basis", "store.rs",
           "        Basis::Pin => declared_max_age && within_age,\n        Basis::Head => false,",
           "        Basis::Pin => declared_max_age && within_age,\n        Basis::Head => declared_max_age && within_age,",
           ("Writfold.Core.Store.code_recorded_iff", "Writfold.Core.Store.code_head_never_recorded")),
    # store: a receipt stands in only while within recorded_max_age (#4)
    Mutant("recorded-ignores-age", "store.rs",
           "        Basis::Pin => declared_max_age && within_age,",
           "        Basis::Pin => declared_max_age,",
           ("Writfold.Core.Store.code_recorded_iff",)),
    # store: a verdict this machine reached beats any receipt (#4)
    Mutant("receipt-beats-local", "store.rs",
           "        | Verdict::Due => false,",
           "        | Verdict::Due => recorded,",
           ("Writfold.Core.Store.code_local_wins", "Writfold.Core.Store.use_recorded_iff")),
    # store: an unverifiable pinned verdict is never recorded (#4)
    Mutant("pin-records-unverifiable", "store.rs",
           "    !matches!(judged, Verdict::Unverifiable)",
           "    true",
           ("Writfold.Core.Store.code_recorded_unverifiable_never",)),
    # order: the statement that stands is the newest, not the oldest (#15)
    Mutant("newest-keeps-oldest", "order.rs",
           "                Some(b) => precedes(items[b].seq, &items[b].name, items[i].seq, &items[i].name),",
           "                Some(b) => precedes(items[i].seq, &items[i].name, items[b].seq, &items[b].name),",
           ("Writfold.Core.Order.newest_loop_spec",)),
    # order: only a statement that stands can be the newest (#15)
    Mutant("newest-ignores-stands", "order.rs",
           "        if items[i].stands {",
           "        if true {",
           ("Writfold.Core.Order.newest_loop_spec",)),
    # subject: another checkout of the KB's repository is never judged
    Mutant("pick-recorded-checkout", "subject.rs",
           "    if recorded && !recorded_is_kb {",
           "    if recorded {",
           ("Writfold.Core.Subject.pick_spec",
            "Writfold.Core.Subject.recorded_never_kb_checkout")),
    # subject: the enclosing repository wins only when it is not the KB's
    Mutant("pick-here-even-kb", "subject.rs",
           "    if here && !here_is_kb {",
           "    if here {",
           ("Writfold.Core.Subject.pick_spec",
            "Writfold.Core.Subject.unchanged_unless_checkout")),
    # subject: a recorded checkout outside any repository is the KB's own
    Mutant("pick-checkout-nothing", "subject.rs",
           "    if recorded {\n        return Pick::Kb;",
           "    if recorded {\n        return Pick::Nothing;",
           ("Writfold.Core.Subject.pick_spec",
            "Writfold.Core.Subject.some_subject")),
    # gate lines: a gate is skipped only when every step naming it was
    Mutant("skipped-any-step", "policy/lines.rs",
           "        if !is_skipped(&owners[index]) {",
           "        if false {",
           ("Writfold.Core.Policy.all_skipped_loop_spec",
            "Writfold.Core.Policy.all_skipped_spec")),
    # gate lines: a gate no step names is not skipped
    Mutant("skipped-unnamed", "policy/lines.rs",
           "    if owners.is_empty() {\n        return false;",
           "    if owners.is_empty() {\n        return true;",
           ("Writfold.Core.Policy.all_skipped_spec",)),
    # gate lines: a gate not skipped is owed until met
    Mutant("owed-ignores-skip", "policy/lines.rs",
           "        if !listed[index].skipped && !listed[index].met {",
           "        if !listed[index].met {",
           ("Writfold.Core.Policy.owed_met_loop_spec",)),
    # gate lines: a stale plan does not close
    Mutant("closes-stale", "policy/lines.rs",
           "    matches!(summary, Summary::Complete) || matches!(summary, Summary::CompleteWithExceptions)",
           "    matches!(summary, Summary::Complete)\n        || matches!(summary, Summary::CompleteWithExceptions)\n        || matches!(summary, Summary::CompleteStale)",
           ("Writfold.Core.Policy.closes_spec",)),
    # gate lines: an invalid receipt or definition keeps the lines unclean
    Mutant("clean-ignores-invalid", "policy/lines.rs",
           "    closes(summary) && owed_met(listed) && valid",
           "    closes(summary) && owed_met(listed)",
           ("Writfold.Core.Policy.lines_clean_iff",)),
    # find: the later tiers stop only when no hit could be shown
    Mutant("full-late", "find.rs",
           "    shown >= limit",
           "    shown > limit",
           ("Writfold.Core.Walk.full_spec", "Writfold.Core.Walk.not_full_admits")),
    # find: the rows walk every tier's hits once, in order (show, 20 November 2026)
    Mutant("show-ignores-seen", "find.rs",
           "        if admit(shown.len(), limit, among(&records, hit.record), hit.passes) {",
           "        if admit(shown.len(), limit, false, hit.passes) {",
           ("Writfold.Core.Show.show_loop_spec",)),
    Mutant("show-records-wrong", "find.rs",
           "            records.push(hit.record);",
           "            records.push(0);",
           ("Writfold.Core.Show.show_loop_spec",)),
    Mutant("among-skips-last", "find.rs",
           "    while i < records.len() {",
           "    while i + 1 < records.len() {",
           ("Writfold.Core.Show.among_loop_spec",)),
    # find: stop after the exact tier only on exactly one record
    Mutant("exact-stops-on-many", "find.rs",
           "    id_shaped && exact_hits == 1",
           "    id_shaped && exact_hits >= 1",
           ("Writfold.Core.Show.exact_stops_spec",)),
    # decision status --set: the flags fit the change (20 November 2026)
    Mutant("flags-actor-inverted", "decision.rs",
           "    if needs.actor && !got.actor {",
           "    if needs.actor && got.actor {",
           ("Writfold.Core.Decision.flags_none_iff", "Writfold.Core.Decision.flags_actor_first")),
    Mutant("flags-commit-not-refused", "decision.rs",
           "    if !needs.commit && got.commit {",
           "    if needs.commit && got.commit {",
           ("Writfold.Core.Decision.flags_none_iff",)),
    Mutant("flags-run-or-repo-ignored", "decision.rs",
           "    if !needs.evidence && got.run_or_repo {",
           "    if needs.evidence && got.run_or_repo {",
           ("Writfold.Core.Decision.flags_none_iff",)),
    # decision status --set: written only when every needed fact holds
    Mutant("writes-ignores-grounded", "decision.rs",
           "        && (!needs.evidence || facts.grounded)",
           "        && (!needs.evidence || facts.on_ref)",
           ("Writfold.Core.Decision.writes_spec",)),
    Mutant("writes-ignores-successor", "decision.rs",
           "        && (!needs.successor || facts.successor)",
           "        && (needs.successor || facts.successor)",
           ("Writfold.Core.Decision.writes_spec",)),
    # checklist: an item the `snapshot` list names needs a look that kept one
    Mutant("covered-ignores-snapshot", "checklist.rs",
           "        Item::Supported => !unsnapped,",
           "        Item::Supported => true,",
           ("Writfold.Core.Checklist.covered_under_spec",)),
    # ---- delivery modes, second round (17 November 2026) ----
    # delivery: an obligation is met under its own policy, not by any met word
    Mutant("delivery-accept-ignored", "delivery.rs",
           "    !crate::policy::met(owed.standing, &owed.accept)",
           "    !owed.standing.met()",
           ("Writfold.Core.Delivery.unmet_spec",)),
    # delivery: ready is every obligation met under its policy, not a met standing
    Mutant("ready-from-standing", "delivery.rs",
           "    if owed.is_empty() {\n        return false;\n    }\n    match first_unmet(owed) {\n        None => true,\n        Some(_) => false,\n    }",
           "    standing(owed).met()",
           ("Writfold.Core.Delivery.ready_iff",)),
    # delivery: refused only when the first unmet obligation stands on a met word
    Mutant("refused-any-unmet", "delivery.rs",
           "        Some(index) => owed[index].standing.met(),",
           "        Some(_) => true,",
           ("Writfold.Core.Delivery.refused_iff",)),
    # delivery: a step's deliver gate is met exactly when the candidate is ready
    Mutant("fact-accepts-when-unready", "delivery.rs",
           "    let taken = ready(owed);",
           "    let taken = true;",
           ("Writfold.Core.Delivery.fact_met_iff_ready",)),
    # delivery: the last obligation counts too
    Mutant("first-unmet-skips-last", "delivery.rs",
           "    while index < owed.len() {\n        if unmet(&owed[index]) {\n            return Some(index);",
           "    while index + 1 < owed.len() {\n        if unmet(&owed[index]) {\n            return Some(index);",
           ("Writfold.Core.Delivery.first_unmet_loop_spec",)),
    # delivery: a withdrawn candidate never decides
    Mutant("deciding-counts-withdrawn", "delivery.rs",
           "        if is_open(&places[index]) {",
           "        if !is_bound(&places[index]) {",
           ("Writfold.Core.Delivery.deciding_loop_spec",)),
    # delivery: a withdrawn candidate is passed over, not the end of the scan
    Mutant("deciding-stops-at-withdrawn", "delivery.rs",
           "        if is_bound(&places[index]) {\n            return None;",
           "        if !is_open(&places[index]) {\n            return None;",
           ("Writfold.Core.Delivery.deciding_loop_spec",)),
    # delivery: posted:<mode> is open until a post record exists
    Mutant("posted-always", "delivery.rs",
           "    if recorded {\n        Standing::Attested",
           "    if true {\n        Standing::Attested",
           ("Writfold.Core.Delivery.posted_spec",)),
    # delivery: a post record is a statement, never a passing check
    Mutant("posted-satisfied", "delivery.rs",
           "        Standing::Attested\n    } else {\n        Standing::Checked(State::Open)",
           "        Standing::Checked(State::Satisfied)\n    } else {\n        Standing::Checked(State::Open)",
           ("Writfold.Core.Delivery.posted_spec", "Writfold.Core.Delivery.posted_never_satisfied")),
    # delivery guard: a postable candidate is same
    Mutant("postable-drops-same", "delivery.rs",
           "    seen.same && seen.ready && seen.live && seen.routed && !seen.spent",
           "    seen.ready && seen.live && seen.routed && !seen.spent",
           ("Writfold.Core.Delivery.postable_spec",)),
    # delivery guard: a postable candidate is ready
    Mutant("postable-drops-ready", "delivery.rs",
           "    seen.same && seen.ready && seen.live && seen.routed && !seen.spent",
           "    seen.same && seen.live && seen.routed && !seen.spent",
           ("Writfold.Core.Delivery.postable_spec",)),
    # delivery guard: a postable candidate is live
    Mutant("postable-drops-live", "delivery.rs",
           "    seen.same && seen.ready && seen.live && seen.routed && !seen.spent",
           "    seen.same && seen.ready && seen.routed && !seen.spent",
           ("Writfold.Core.Delivery.postable_spec",)),
    # delivery guard: a postable candidate is routed
    Mutant("postable-drops-routed", "delivery.rs",
           "    seen.same && seen.ready && seen.live && seen.routed && !seen.spent",
           "    seen.same && seen.ready && seen.live && !seen.spent",
           ("Writfold.Core.Delivery.postable_spec",)),
    # delivery guard: a posted candidate is spent
    Mutant("postable-drops-spent", "delivery.rs",
           "    seen.same && seen.ready && seen.live && seen.routed && !seen.spent",
           "    seen.same && seen.ready && seen.live && seen.routed",
           ("Writfold.Core.Delivery.postable_spec",)),
    # delivery guard: known bytes need one postable candidate, not every candidate ready
    Mutant("guard-known-all", "delivery.rs",
           "        Body::Known => guard_any(seen),",
           "        Body::Known => guard_any(seen) && guard_all(seen),",
           ("Writfold.Core.Delivery.guard_allow_iff",)),
    # delivery guard: unread bytes under a route never pass
    Mutant("guard-unread-deny-allows", "delivery.rs",
           "            Unread::Deny => false,",
           "            Unread::Deny => guard_all(seen),",
           ("Writfold.Core.Delivery.guard_allow_iff", "Writfold.Core.Delivery.guard_deny_unread")),
    # delivery guard: a withdrawn candidate need not be ready
    Mutant("guard-all-counts-withdrawn", "delivery.rs",
           "        if seen[index].live && !seen[index].ready {",
           "        if !seen[index].ready {",
           ("Writfold.Core.Delivery.guard_all_loop_spec",)),
    # delivery guard: the last candidate counts for the all-ready rule
    Mutant("guard-all-skips-last", "delivery.rs",
           "    while index < seen.len() {\n        if seen[index].live && !seen[index].ready {",
           "    while index + 1 < seen.len() {\n        if seen[index].live && !seen[index].ready {",
           ("Writfold.Core.Delivery.guard_all_loop_spec",)),
    # delivery guard: the last candidate may hold the bytes
    Mutant("guard-any-skips-last", "delivery.rs",
           "    while index < seen.len() {\n        if postable(&seen[index]) {",
           "    while index + 1 < seen.len() {\n        if postable(&seen[index]) {",
           ("Writfold.Core.Delivery.guard_any_loop_spec",)),
    # ---- end delivery modes, second round ----
    # ---- the current run of a seat (17 November 2026) ----
    # pointer: a session pointer decides before the work tree's
    Mutant("level-tree-before-session", "pointer.rs",
           "    if is_set(session) {\n        return Some(Level::Session);\n    }\n    if is_set(tree) {\n        return Some(Level::Tree);\n    }",
           "    if is_set(tree) {\n        return Some(Level::Tree);\n    }\n    if is_set(session) {\n        return Some(Level::Session);\n    }",
           ("Writfold.Core.Pointer.level_spec",
            "Writfold.Core.Pointer.open_spec",
            "Writfold.Core.Pointer.session_decides",
            "Writfold.Core.Pointer.ended_not_skipped")),
    # pointer: a pointer to a run the KB lacks is set, and so not skipped
    Mutant("is-set-skips-gone", "pointer.rs",
           "    !matches!(one, Pointer::Unset)",
           "    !matches!(one, Pointer::Unset | Pointer::Gone)",
           ("Writfold.Core.Pointer.is_set_spec",
            "Writfold.Core.Pointer.level_spec",
            "Writfold.Core.Pointer.open_spec",
            "Writfold.Core.Pointer.session_decides",
            "Writfold.Core.Pointer.tree_decides",
            "Writfold.Core.Pointer.ended_not_skipped")),
    # pointer: a closed run at the deciding level is not skipped for an open one below
    Mutant("open-skips-closed-session", "pointer.rs",
           "        Some(Level::Session) => matches!(session, Pointer::Open),",
           "        Some(Level::Session) => matches!(session, Pointer::Open) || matches!(tree, Pointer::Open),",
           ("Writfold.Core.Pointer.open_spec",
            "Writfold.Core.Pointer.session_decides",
            "Writfold.Core.Pointer.ended_not_skipped")),
    # pointer: only an open run is an open current run
    Mutant("open-shared-any-run", "pointer.rs",
           "        Some(Level::Shared) => matches!(shared, Pointer::Open),",
           "        Some(Level::Shared) => true,",
           ("Writfold.Core.Pointer.open_spec",
            "Writfold.Core.Pointer.shared_alone",
            "Writfold.Core.Pointer.ended_not_skipped")),
    # pointer: the shared pointer is always written
    Mutant("writes-shared-only-without-session", "pointer.rs",
           "        shared: true,",
           "        shared: !has_session,",
           ("Writfold.Core.Pointer.writes_spec",)),
    # pointer: a seat with a session writes its session pointer
    Mutant("writes-no-session", "pointer.rs",
           "        session: has_session,",
           "        session: false,",
           ("Writfold.Core.Pointer.writes_spec",
            "Writfold.Core.Pointer.writer_reads_its_run")),
    # ---- plan repin (17 November 2026) ----
    # repin: the entry in force is one that applies
    Mutant("in-force-inverted", "policy/repin.rs",
           "        if applies[index] {",
           "        if !applies[index] {",
           ("Writfold.Core.Policy.in_force_loop_spec",)),
    # repin: the oldest entry counts when no newer one applies
    Mutant("in-force-skips-oldest", "policy/repin.rs",
           "    while index > 0 {",
           "    while index > 1 {",
           ("Writfold.Core.Policy.in_force_loop_spec",)),
    # repin: a closed run takes no repin
    Mutant("repin-closed-run", "policy/repin.rs",
           "    if !writable {",
           "    if false {",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff",
            "Writfold.Core.Policy.repin_closed_refused")),
    # repin: a pin file that does not read is not adopted
    Mutant("repin-ignores-invalid", "policy/repin.rs",
           "    if !valid {",
           "    if false {",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff")),
    # repin: refused when nothing moved
    Mutant("repin-when-same", "policy/repin.rs",
           "        Pin::Same => Err(Refusal::Same),",
           "        Pin::Same => Ok(()),",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff")),
    # repin: refused when the run pinned nothing
    Mutant("repin-when-unpinned", "policy/repin.rs",
           "        Pin::Unpinned => Err(Refusal::Unpinned),",
           "        Pin::Unpinned => Ok(()),",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff")),
    # repin: a gate an open step owes must load to be adopted
    Mutant("repin-undefined-gate", "policy/repin.rs",
           "            if !loads {",
           "            if false {",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff")),
    # repin: adopting a lock list never reopens a skipped step
    Mutant("repin-reopens-skip", "policy/repin.rs",
           "            if !keeps {",
           "            if false {",
           ("Writfold.Core.Policy.repin_check_spec", "Writfold.Core.Policy.repin_ok_iff")),
    # ---- end plan repin ----
    # ---- attached stores (18 November 2026) ----
    # store: the pin judges only a citation that names a revision
    Mutant("basis-ignores-rev", "store.rs",
           "    if declared && pinned && has_rev {",
           "    if declared && pinned {",
           ("Writfold.Core.Store.basis_pin_iff",)),
    # store: a clone that is behind is not evidence of loss
    Mutant("pinned-absent-gone", "store.rs",
           "        Commit::Absent => Verdict::Unverifiable,",
           "        Commit::Absent => Verdict::Gone,",
           ("Writfold.Core.Store.pinned_behind_unverifiable",)),
    # store: a commit no ref reaches is not fresh
    Mutant("pinned-unanchored-fresh", "store.rs",
           "        Commit::Unanchored => Verdict::Gone,",
           "        Commit::Unanchored => Verdict::Fresh,",
           ("Writfold.Core.Store.pinned_fresh_iff",
            "Writfold.Core.Store.pinned_unanchored_gone")),
    # store: other bytes at the pin are not the cited ones
    Mutant("pinned-other-bytes-fresh", "store.rs",
           "            AtPin::Cited => Verdict::Fresh,\n            AtPin::Other | AtPin::Nothing => Verdict::Gone,",
           "            AtPin::Cited | AtPin::Other => Verdict::Fresh,\n            AtPin::Nothing => Verdict::Gone,",
           ("Writfold.Core.Store.pinned_fresh_iff",
            "Writfold.Core.Store.pinned_other_bytes")),
    # store: a committed hint needs the declared origin
    Mutant("resolve-hint-any-origin", "store.rs",
           "    if matches!(hint, Candidate::Clone(true)) {",
           "    if matches!(hint, Candidate::Clone(_)) {",
           ("Writfold.Core.Store.resolve_committed_needs_origin",
            "Writfold.Core.Store.resolve_mismatch_iff")),
    # store: the map's clone is flagged when its origin differs
    Mutant("resolve-map-unflagged", "store.rs",
           "            flagged: !*origin,",
           "            flagged: false,",
           ("Writfold.Core.Store.resolve_map_wins",)),
    # store: search roots before committed hints
    Mutant("resolve-hint-before-search", "store.rs",
           "    if matches!(search, Candidate::Clone(true)) {\n        return Resolved::Found {\n            via: Via::SearchRoot,\n            flagged: false,\n        };\n    }\n    if matches!(hint, Candidate::Clone(true)) {\n        return Resolved::Found {\n            via: Via::Hint,",
           "    if matches!(hint, Candidate::Clone(true)) {\n        return Resolved::Found {\n            via: Via::Hint,\n            flagged: false,\n        };\n    }\n    if matches!(search, Candidate::Clone(true)) {\n        return Resolved::Found {\n            via: Via::SearchRoot,",
           ("Writfold.Core.Store.resolve_search_before_hint",)),
    # store: an artifact needs a full commit ID
    Mutant("artifact-short-rev-admitted", "store.rs",
           "    if !full_rev {\n",
           "    if !full_rev && false {\n",
           ("Writfold.Core.Store.artifact_admit_iff", "Writfold.Core.Store.artifact_short_rev")),
    # store: an unanchored commit is refused, never admitted
    Mutant("artifact-unanchored-admitted", "store.rs",
           "        Commit::Unanchored => Artifact::Refuse(Refusal::Unanchored),",
           "        Commit::Unanchored => Artifact::Admit,",
           ("Writfold.Core.Store.artifact_admit_iff", "Writfold.Core.Store.artifact_admit_reads_fresh", "Writfold.Core.Store.artifact_unanchored")),
    # store: the declaration is checked before the clone
    Mutant("artifact-found-before-declared", "store.rs",
           "    if !declared {\n        return Artifact::Refuse(Refusal::Undeclared);\n    }\n    if !found {\n        return Artifact::Refuse(Refusal::NotLocal);\n    }",
           "    if !found {\n        return Artifact::Refuse(Refusal::NotLocal);\n    }\n    if !declared {\n        return Artifact::Refuse(Refusal::Undeclared);\n    }",
           ("Writfold.Core.Store.artifact_undeclared",)),
    # store: a missing commit says fetch, not push
    Mutant("artifact-absent-as-unanchored", "store.rs",
           "        Commit::Absent => Artifact::Refuse(Refusal::NoCommit),",
           "        Commit::Absent => Artifact::Refuse(Refusal::Unanchored),",
           ("Writfold.Core.Store.artifact_no_commit",)),
    # store: an alias of two records lists the directory under neither
    Mutant("claim-two-listed", "store.rs",
           "    if aliased == 1 {\n",
           "    if aliased >= 1 {\n",
           ("Writfold.Core.Store.claim_listed_iff", "Writfold.Core.Store.claim_ambiguous_iff")),
    # store: a subject that names no record lists nothing
    Mutant("claim-unknown-listed", "store.rs",
           "    } else if aliased == 0 {\n        Claim::Unknown\n",
           "    } else if aliased == 0 {\n        Claim::Listed\n",
           ("Writfold.Core.Store.claim_listed_iff", "Writfold.Core.Store.claim_unknown_iff")),
    # store: a shared alias never hides a record's own ID
    Mutant("claim-alias-hides-id", "store.rs",
           "    if exact {\n        return Claim::Listed;\n",
           "    if exact && aliased < 2 {\n        return Claim::Listed;\n",
           ("Writfold.Core.Store.claim_exact_wins", "Writfold.Core.Store.claim_listed_iff", "Writfold.Core.Store.claim_ambiguous_iff")),
    # store: nothing is listed from a repository that is not a declared store
    Mutant("claim-any-repository", "store.rs",
           "    if !attached {\n",
           "    if !attached && false {\n",
           ("Writfold.Core.Store.claim_needs_store", "Writfold.Core.Store.claim_listed_iff", "Writfold.Core.Store.claim_ambiguous_iff", "Writfold.Core.Store.claim_unknown_iff")),
    # store: an uncommitted snapshot is never cited
    Mutant("snapshot-uncommitted-recorded", "store.rs",
           '        (Commit::Absent, _) | (_, AtPin::Nothing) => Snapshot::Withhold(Withheld::NotCommitted),\n        (_, AtPin::Other) => Snapshot::Withhold(Withheld::OtherBytes),\n        (Commit::Unanchored, AtPin::Cited) => Snapshot::Withhold(Withheld::Unanchored),\n        (Commit::Anchored, AtPin::Cited) => Snapshot::Record,\n',
           '        (Commit::Absent, _) => Snapshot::Withhold(Withheld::NotCommitted),\n        (_, AtPin::Other) => Snapshot::Withhold(Withheld::OtherBytes),\n        (Commit::Unanchored, AtPin::Cited | AtPin::Nothing) => Snapshot::Withhold(Withheld::Unanchored),\n        (Commit::Anchored, AtPin::Cited | AtPin::Nothing) => Snapshot::Record,\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_record_is_fresh", "Writfold.Core.Store.snapshot_uncommitted_withheld", "Writfold.Core.Store.snapshot_order", "Writfold.Core.Store.snapshot_stage_iff")),
    # store: a snapshot no ref reaches is not recorded
    Mutant("snapshot-unanchored-recorded", "store.rs",
           '        (Commit::Unanchored, AtPin::Cited) => Snapshot::Withhold(Withheld::Unanchored),\n',
           '        (Commit::Unanchored, AtPin::Cited) => Snapshot::Record,\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_record_is_fresh", "Writfold.Core.Store.snapshot_order")),
    # store: a snapshot whose committed bytes differ is not recorded
    Mutant("snapshot-other-bytes-recorded", "store.rs",
           '        (_, AtPin::Other) => Snapshot::Withhold(Withheld::OtherBytes),\n',
           '        (Commit::Anchored, AtPin::Other) => Snapshot::Record,\n        (_, AtPin::Other) => Snapshot::Withhold(Withheld::OtherBytes),\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_record_is_fresh", "Writfold.Core.Store.snapshot_order")),
    # store: a snapshot is kept only in a pinned store
    Mutant("snapshot-head-store", "store.rs",
           '    if !facts.pinned {\n',
           '    if !facts.pinned && false {\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_order", "Writfold.Core.Store.snapshot_stage_iff")),
    # store: bytes that no longer match their provenance are not recorded
    Mutant("snapshot-damaged-recorded", "store.rs",
           '    if !facts.intact {\n',
           '    if !facts.intact && false {\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_order", "Writfold.Core.Store.snapshot_stage_iff")),
    # store: a status that embeds nothing takes no snapshot
    Mutant("snapshot-any-status", "store.rs",
           '    if !facts.embeds {\n',
           '    if !facts.embeds && false {\n',
           ("Writfold.Core.Store.snapshot_record_iff", "Writfold.Core.Store.snapshot_order", "Writfold.Core.Store.snapshot_stage_iff")),
    # store: a head-judged store is named before a missing clone
    Mutant("snapshot-order-swapped", "store.rs",
           '    if !facts.pinned {\n        return Snapshot::Withhold(Withheld::NotPinned);\n    }\n    if !facts.local {\n        return Snapshot::Withhold(Withheld::NotLocal);\n    }\n',
           '    if !facts.local {\n        return Snapshot::Withhold(Withheld::NotLocal);\n    }\n    if !facts.pinned {\n        return Snapshot::Withhold(Withheld::NotPinned);\n    }\n',
           ("Writfold.Core.Store.snapshot_order",)),
    # bind: a look recorded for another item never answers an item that wants its own
    Mutant("own-accepts-other", "bind.rs",
           "            Bound::Named | Bound::This => true,\n            Bound::Other | Bound::Unbound => false,",
           "            Bound::Named | Bound::This | Bound::Other => true,\n            Bound::Unbound => false,",
           ("Writfold.Core.Bind.answers_spec",)),
    # bind: only a named gate answers an item that names its evidence
    Mutant("named-accepts-this", "bind.rs",
           "            Bound::Named => true,\n            Bound::This | Bound::Other | Bound::Unbound => false,",
           "            Bound::Named | Bound::This => true,\n            Bound::Other | Bound::Unbound => false,",
           ("Writfold.Core.Bind.answers_spec",)),
    # bind: an ungrounded reference never answers
    Mutant("answered-ignores-grounded", "bind.rs",
           "        if refs[at].grounded && answers(need, refs[at].bound) {",
           "        if answers(need, refs[at].bound) {",
           ("Writfold.Core.Bind.answered_from_loop_spec",)),
    # bind: an unanswered supported row is stale
    Mutant("item-under-ignores-answer", "bind.rs",
           "            if answered {\n                base",
           "            if true {\n                base",
           ("Writfold.Core.Bind.item_under_spec",)),
    # pin: an unreadable pin never holds
    Mutant("holds-skips-unreadable", "pin.rs",
           "            Pin::Held => {}\n            Pin::Moved | Pin::Aged | Pin::Unreadable => return false,",
           "            Pin::Held | Pin::Unreadable => {}\n            Pin::Moved | Pin::Aged => return false,",
           ("Writfold.Core.Pin.holds_from_loop_spec",)),
    # pin: a catalog default lifetime ages like a token's own
    Mutant("timed-default-never-ages", "pin.rs",
           "        Lifetime::Immutable | Lifetime::Never => Pin::Held,\n        Lifetime::Token(ttl) | Lifetime::Default(ttl) => {",
           "        Lifetime::Immutable | Lifetime::Never | Lifetime::Default(_) => Pin::Held,\n        Lifetime::Token(ttl) => {",
           ("Writfold.Core.Pin.timed_spec",)),
    # pin: a negative look's pins decide whether it still found nothing
    Mutant("nothing-found-ignores-pins", "pin.rs",
           "        Status::NoData | Status::NotFound => holds(pins),",
           "        Status::NoData | Status::NotFound => true,",
           ("Writfold.Core.Pin.nothing_found_spec",)),
    # pin: pins never make a receipt about other bytes current
    Mutant("current-under-ignores-about", "pin.rs",
           "    about && holds(pins)",
           "    holds(pins)",
           ("Writfold.Core.Pin.current_under_spec",)),
    # the total theorems (no panic for any input) catch an out-of-bounds read
    Mutant("answered-reads-past-end", "bind.rs",
           "        if refs[at].grounded && answers(need, refs[at].bound) {",
           "        if refs[at + 1].grounded && answers(need, refs[at].bound) {",
           ("Writfold.Core.Bind.answered_from_loop_spec", "Writfold.Core.Bind.answered_from_total")),
    Mutant("holds-reads-past-end", "pin.rs",
           "        match pins[at] {",
           "        match pins[at + 1] {",
           ("Writfold.Core.Pin.holds_from_loop_spec", "Writfold.Core.Pin.holds_from_total")),
    # the total theorems of the four helpers rewritten with checked access
    # (docs/writfold/tauten-adoption.md) catch what their precondition specs
    # cannot: a read or an overflow outside the precondition
    # idle: a position past the end is read
    Mutant("newer-reads-past-end", "idle.rs",
           "        None => false,",
           "        None => stamps[i] > 0,",
           ("Writfold.Core.Idle.newer_total", "Writfold.Core.Idle.newer_past_end")),
    # recorded: an index past the end picks the last statement
    Mutant("better-past-end-underflows", "verdict/recorded.rs",
           "        None => return best,",
           "        None => return Some(recorded.len() - 1),",
           ("Writfold.Core.Recorded.better_total", "Writfold.Core.Recorded.better_past_end")),
    # scope: a `*` stretch from a cursor outside the pattern overflows
    Mutant("char-step-stretches-out", "scope.rs",
           "    if c.starred && c.at < pattern.len() && c.from < text.len() {",
           "    if c.starred {",
           ("Writfold.Core.Scope.char_step_total", "Writfold.Core.Scope.char_step_outside")),
    # scope: a `**` stretch from a cursor outside the path overflows
    Mutant("path-step-stretches-out", "scope.rs",
           "    if c.starred && c.at < wanted.len() && c.from < parts.len() {",
           "    if c.starred {",
           ("Writfold.Core.Scope.path_step_total", "Writfold.Core.Scope.path_step_outside")),
    # skips-last mutants for the loops whose theorems no mutant broke, added so
    # their functions can be tauten claims (docs/writfold/tauten-adoption.md)
    # checklist: one stale entry is enough, the last one too
    Mutant("any-stale-skips-last", "checklist.rs",
           "    while index < entries.len() {\n        if stale(&entries[index]) {",
           "    while index + 1 < entries.len() {\n        if stale(&entries[index]) {",
           ("Writfold.Core.Checklist.any_stale_loop_spec",)),
    # idle: the newest stamp may be the last one
    Mutant("newest-skips-last", "idle.rs",
           "    while i < stamps.len() {\n        if newer(stamps, best, i) {",
           "    while i + 1 < stamps.len() {\n        if newer(stamps, best, i) {",
           ("Writfold.Core.Idle.newest_loop_spec",)),
    # policy: an unpinned last gate counts
    Mutant("any-unpinned-skips-last", "policy.rs",
           "    while index < pins.len() {\n        if is_unpinned(&pins[index]) {",
           "    while index + 1 < pins.len() {\n        if is_unpinned(&pins[index]) {",
           ("Writfold.Core.Policy.any_unpinned_loop_spec",)),
    # policy: a waived last gate counts
    Mutant("any-waived-skips-last", "policy.rs",
           "    while index < met.len() {\n        if how_waived(&met[index]) {",
           "    while index + 1 < met.len() {\n        if how_waived(&met[index]) {",
           ("Writfold.Core.Policy.any_waived_loop_spec",)),
    # policy: an attested last gate counts
    Mutant("any-attested-skips-last", "policy.rs",
           "    while index < met.len() {\n        if how_attested(&met[index]) {",
           "    while index + 1 < met.len() {\n        if how_attested(&met[index]) {",
           ("Writfold.Core.Policy.any_attested_loop_spec",)),
    # policy: an open last step counts
    Mutant("any-open-skips-last", "policy.rs",
           "    while index < closes.len() {\n        if is_open(&closes[index]) {",
           "    while index + 1 < closes.len() {\n        if is_open(&closes[index]) {",
           ("Writfold.Core.Policy.any_open_loop_spec",)),
    # policy: the last step must have passed too
    Mutant("all-passed-skips-last", "policy.rs",
           "    while index < closes.len() {\n        if !is_passed(&closes[index]) {",
           "    while index + 1 < closes.len() {\n        if !is_passed(&closes[index]) {",
           ("Writfold.Core.Policy.all_passed_loop_spec",)),
    # recorded: the last recorded statement is a candidate
    Mutant("best-recorded-skips-last", "verdict/recorded.rs",
           "    while index < recorded.len() {\n        best = better(recorded, best, index);",
           "    while index + 1 < recorded.len() {\n        best = better(recorded, best, index);",
           ("Writfold.Core.Recorded.best_recorded_loop_spec",)),
)

@dataclass(frozen=True)
class CoreMutant:
    """A mutation of a definition under proofs/Calculus (#205): no
    translation step; the Calculus library is built against it."""
    name: str
    file: str
    before: str
    after: str
    fails: tuple[str, ...]


CALCULUS = PROOFS / "Calculus"

CORE_MUTANTS = (
    CoreMutant("admits-unknown", "Word.lean",
               "def admits : Tri → Bool\n  | yes => true\n  | _ => false",
               "def admits : Tri → Bool\n  | yes => true\n  | unknown => true\n  | _ => false",
               ("Calculus.Tri.admits_iff", "Calculus.Tri.unknown_mem_never_admits", "Calculus.Tri.unknown_never_admits")),
    CoreMutant("and-unknown-beats-no", "Word.lean",
               "  | no, _ => no\n  | _, no => no\n  | unknown, _ => unknown\n",
               "  | unknown, _ => unknown\n  | no, _ => no\n  | _, no => no\n",
               ("Calculus.Tri.and_comm", "Calculus.Tri.and_eq_no")),
    CoreMutant("all-empty-unknown", "Word.lean",
               "def all (ts : List Tri) : Tri := ts.foldr and yes",
               "def all (ts : List Tri) : Tri := ts.foldr and unknown",
               ("Calculus.Tri.all_append", "Calculus.Tri.all_eq_yes")),
    CoreMutant("join-ask-beats-deny", "Word.lean",
               "  | deny, _ => deny\n  | _, deny => deny\n  | ask, _ => ask\n",
               "  | ask, _ => ask\n  | deny, _ => deny\n  | _, deny => deny\n",
               ("Calculus.Decision.join_comm",)),
    CoreMutant("admit-ignores-unknown", "Word.lean",
               "  if (Tri.all reqs).admits then .pass else action.decision",
               "  if Tri.all reqs != .no then .pass else action.decision",
               ("Calculus.admit_pass_iff",)),
    CoreMutant("remainder-counts-declared-out", "Account.lean",
               "def remainder (ms : List Member) : Nat := ms.countP (· = .remainder)",
               "def remainder (ms : List Member) : Nat := ms.countP (· ≠ .judged)",
               ("Calculus.declared_out_never_counts", "Calculus.remainder_counts", "Calculus.whole_iff_no_remainder")),
    CoreMutant("whole-ignores-remainder", "Account.lean",
               "def whole (ms : List Member) : Bool := remainder ms == 0",
               "def whole (ms : List Member) : Bool := remainder ms == remainder ms",
               ("Calculus.whole_counts", "Calculus.whole_iff_no_remainder")),
    CoreMutant("universal-skips-whole", "Account.lean",
               "  else if !whole (ms.map (·.1)) then .unknown\n",
               "",
               ("Calculus.universal_yes_iff",)),
    CoreMutant("universal-empty-yes", "Account.lean",
               "  else if (judgedOf ms).isEmpty && !allowEmpty then .unknown\n",
               "",
               ("Calculus.universal_yes_iff",)),
    CoreMutant("reported-accepted", "Evidence.lean",
               "  | .waived => a.waived\n  | _ => false",
               "  | .waived => a.waived\n  | .reported => a.waived\n  | _ => false",
               ("Calculus.reported_never_met",)),
    CoreMutant("attested-reads-satisfied", "Evidence.lean",
               "    | .attested, _ => .attested",
               "    | .attested, _ => .satisfied",
               ("Calculus.class_never_promoted", "Calculus.satisfied_only_by_current_pass")),
    CoreMutant("inf-is-or", "Evidence.lean",
               "  ⟨a.satisfied && b.satisfied, a.attested && b.attested, a.waived && b.waived⟩",
               "  ⟨a.satisfied || b.satisfied, a.attested || b.attested, a.waived || b.waived⟩",
               ("Calculus.inf_assoc", "Calculus.inf_comm", "Calculus.inf_top", "Calculus.met_inf")),
    CoreMutant("deciding-takes-oldest", "Evidence.lean",
               "  (rs.filter counts).foldl\n    (fun best r => match best with\n      | none => some r\n      | some b => if seq b ≤ seq r then some r else some b) none",
               "  (rs.filter counts).foldl\n    (fun best r => match best with\n      | none => some r\n      | some b => if seq r ≤ seq b then some r else some b) none",
               ("Calculus.deciding_is_newest", "Calculus.deciding_mem", "Calculus.deciding_none_iff")),
    CoreMutant("deciding-counts-unbound", "Evidence.lean",
               "  (rs.filter counts).foldl\n    (fun best r => match best with\n      | none => some r\n      | some b => if seq b ≤ seq r then some r else some b) none",
               "  rs.foldl\n    (fun best r => match best with\n      | none => some r\n      | some b => if seq b ≤ seq r then some r else some b) none",
               ("Calculus.deciding_is_newest", "Calculus.deciding_mem", "Calculus.deciding_none_iff", "Calculus.uncounted_never_decides")),
    CoreMutant("undecided-passes-closed", "Site.lean",
               "  | .ask => .ask\n  | .deny => .deny\n\ndef Site.decide",
               "  | .ask => .pass\n  | .deny => .deny\n\ndef Site.decide",
               ("Calculus.closed_pass_iff",)),
    CoreMutant("posture-overrides-judged", "Site.lean",
               "  | .judged d => d\n",
               "  | .judged _ => p.undecided\n",
               ("Calculus.closed_pass_iff", "Calculus.judged_unchanged", "Calculus.open_is_declared", "Calculus.stricter_never_loosens")),
    CoreMutant("stricter-takes-first", "Site.lean",
               "  | .deny, _ | _, .deny => .deny\n  | .ask, _ | _, .ask => .ask\n  | .open, .open => .open",
               "  | a, _ => a",
               ("Calculus.stricter_comm", "Calculus.stricter_open_left")),
    CoreMutant("resolve-first-wins", "Referent.lean",
               "    | .some r => if rest.all (fun c' => c'.referent == some r) then .one r else .ambiguous",
               "    | .some r => .one r",
               ("Calculus.resolve_one_agree", "Calculus.resolve_one_mem")),
    CoreMutant("resolve-falls-back", "Referent.lean",
               "  match cs.filter (Candidate.top cs) with",
               "  match cs with",
               ("Calculus.resolve_one_agree", "Calculus.resolve_one_mem")),
    CoreMutant("resolve-ignores-missing-offer", "Referent.lean",
               "if rest.all (fun c' => c'.referent == some r) then",
               "if rest.all (fun c' => c'.referent == some r || c'.referent == none) then",
               ("Calculus.resolve_one_agree",)),
    CoreMutant("settle-keeps-change-behind", "Referent.lean",
               "  else if ws.claimsChange w && !refutable then ws.unknown\n",
               "",
               ("Calculus.change_needs_refutable",)),
    CoreMutant("settle-judges-unresolved", "Referent.lean",
               "  if !resolved then ws.unknown\n",
               "  if false then ws.unknown\n",
               ("Calculus.change_needs_refutable", "Calculus.judge_needs", "Calculus.unresolved_never_judged")),
    CoreMutant("settle-drops-fresh", "Referent.lean",
               "  else if ws.claimsChange w && !refutable then ws.unknown\n  else w",
               "  else if ws.claimsChange w && !refutable then ws.unknown\n  else ws.unknown",
               ("Calculus.settle_identity", "Calculus.unclaimed_survives")),
    CoreMutant("refines-up-to-unknown", "Knowledge.lean",
               "def Words.refines (v w : W) : Prop := v = w ∨ v = ws.unknown",
               "def Words.refines (v w : W) : Prop := v = w ∨ w = ws.unknown",
               ("Calculus.Words.refines_trans", "Calculus.Words.unknown_refines",
                "Calculus.Words.refines_unknown_iff", "Calculus.Words.told_refines",
                "Calculus.Words.refines_disagree", "Calculus.Tri.words_refines_iff",
                "Calculus.settle_monotone")),
    CoreMutant("unknown-told", "Knowledge.lean",
               "def Words.told (w : W) : Prop := w ≠ ws.unknown",
               "def Words.told (w : W) : Prop := True",
               ("Calculus.Words.told_refines", "Calculus.Words.unknown_never_told")),
    CoreMutant("search-partial-says-no", "Account.lean",
               "  if found then .yes else if isWhole then .no else .unknown",
               "  if found then .yes else .no",
               ("Calculus.absence_needs_whole",)),
)

# lake reports `error: <path>.lean:<line>:<col>: <message>`
ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
DECLARATION = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)", re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)


def declarations(path: Path) -> list[tuple[int, str]]:
    """(line, fully qualified name) of every declaration in a theorem file."""
    stack: list[str] = []
    found = []
    # Lean reports a declaration's error at the start of its doc comment.
    doc: int | None = None
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if match := NAMESPACE.match(line):
            stack.append(match.group("name"))
            doc = None
            continue
        if line.startswith("end ") and stack:
            stack.pop()
            doc = None
            continue
        if line.startswith("/--") and doc is None:
            doc = number
        if match := DECLARATION.match(line):
            prefix = "".join(f"{n}." for n in stack)
            found.append((doc or number, prefix + match.group("name")))
            doc = None
        elif not line.strip():
            doc = None
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
        if relative.parts[0] not in ("Writfold", "Calculus") or not file.exists():
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
                          text=True, timeout=1800, env=dict(os.environ, **THREADS))


# Mutants run side by side; each build's Lean threads share the cores (#95).
JOBS = max(1, int(os.environ.get("WRITFOLD_MUTANT_JOBS", "6") or "6"))
THREADS = {} if "LEAN_NUM_THREADS" in os.environ else \
    {"LEAN_NUM_THREADS": str(max(1, (os.cpu_count() or 1) // JOBS))}


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
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Writfold.lean",
                 "Calculus.lean", "Audit.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(PROOFS / "Writfold", project / "Writfold")
    shutil.copytree(CALCULUS, project / "Calculus")
    shutil.copytree(scratch / "Extracted", project / "Extracted")
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")
    # The passing build's outputs (#95): Lake rebuilds a module only when its
    # source or an import changed, so the theorem files that import only
    # modules the mutant left byte-identical elaborate exactly as they did
    # there, and are not re-elaborated. A real copy: Lake rewrites in place.
    if (PROOFS / ".lake" / "build").is_dir():
        shutil.copytree(PROOFS / ".lake" / "build", project / ".lake" / "build", symlinks=True)

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


def calibrate_core(mutant: CoreMutant, scratch: Path) -> None:
    """A core mutant: the Calculus library alone, built against the mutated
    definition; the named theorems, and only they, must fail."""
    project = scratch / "proofs"
    project.mkdir()
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Calculus.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(CALCULUS, project / "Calculus")
    source = project / "Calculus" / mutant.file
    text = source.read_text()
    if text.count(mutant.before) != 1:
        raise SystemExit(f"FAIL {mutant.name}: mutation site does not match exactly once")
    source.write_text(text.replace(mutant.before, mutant.after))
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")
    build = run(["lake", "build", "Calculus"], project)
    if build.returncode == 0:
        raise SystemExit(f"FAIL {mutant.name}: the calculus still builds against the mutation")
    failures, unattributed = failing_theorems(build.stdout, project)
    if unattributed:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: errors outside the theorem files: {unattributed}")
    expected = set(mutant.fails)
    if failures != expected:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: expected {sorted(expected)} to fail, got {sorted(failures)}")
    print(f"calibration: {mutant.name} rejected by {', '.join(sorted(failures))}", flush=True)


# Calculus claims proved from other claims, not by unfolding a definition:
# a mutated definition breaks the claims they follow from, and these still
# build. Each names what it follows from, so the list is reviewed with it.
DERIVED = {
    "Calculus.Tri.le_refl": "decided over the three words of leb",
    "Calculus.Tri.le_trans": "decided over the three words of leb",
    "Calculus.Tri.le_antisymm": "decided over the three words of leb",
    "Calculus.Tri.refines_only_unknown": "decided over the three words of leb",
    "Calculus.Tri.and_assoc": "decided over the table of and",
    "Calculus.Tri.and_idem": "decided over the table of and",
    "Calculus.Tri.and_yes": "decided over the table of and",
    "Calculus.Tri.and_mono": "decided over the tables of and and leb",
    "Calculus.Tri.and_eq_yes": "decided over the table of and",
    "Calculus.Tri.all_eq_no": "and_eq_no",
    "Calculus.Tri.all_mono": "and_mono",
    "Calculus.Tri.more_never_admits": "all_append, admits_iff",
    "Calculus.Decision.join_assoc": "decided over the table of join",
    "Calculus.Decision.join_idem": "decided over the table of join",
    "Calculus.Decision.pass_join": "decided over the table of join",
    "Calculus.Decision.join_eq_pass": "decided over the table of join",
    "Calculus.unknown_never_passes": "admit_pass_iff",
    "Calculus.remainder_append": "List.countP_append",
    "Calculus.whole_append": "remainder_append",
    "Calculus.judged_never_counts": "the definition of remainder",
    "Calculus.remainder_never_whole": "universal_yes_iff, whole_iff_no_remainder",
    "Calculus.empty_never_yes": "universal_yes_iff",
    "Calculus.universal_sound": "universal_yes_iff",
    "Calculus.universal_no_of_witness": "the first branch of universal",
    "Calculus.search_yes_iff": "the first branch of search",
    "Calculus.met_narrows": "decided over the standings and the policy's three fields",
    "Calculus.unknown_never_met": "the definition of met",
    "Calculus.inf_idem": "decided over the policy's three fields",
    "Calculus.older_never_outvotes": "deciding_is_newest",
    "Calculus.posture_only_narrows": "decided over the bases and postures",
    "Calculus.stricter_assoc": "decided over the postures",
    "Calculus.resolve_nil": "the definition of resolve",
    "Calculus.resolve_order_free": "resolve_one_mem, resolve_one_agree",
    "Calculus.settle_only_withholds": "decided over the definition of settle",
    "Calculus.settle_refines": "decided over the definition of settle",
    "Calculus.unresolved_unknown": "the definition of about",
    "Calculus.Words.refines_refl": "the first disjunct of refines",
}


def unnamed_laws() -> list[str]:
    """Calculus claims neither named by a core mutant nor listed as derived
    (#205): a law nothing can break in the gate proves nothing."""
    claims = set(re.findall(r"``(Calculus\.[\w.]+)", (PROOFS / "Audit.lean").read_text()))
    named = {name for m in CORE_MUTANTS for name in m.fails}
    stray = sorted(set(DERIVED) & named) + sorted(set(DERIVED) - claims)
    return stray + sorted(claims - named - set(DERIVED))


def foreign_imports() -> list[str]:
    """An import under proofs/Calculus that is not `Calculus.*` (#205)."""
    bad = []
    for path in sorted([PROOFS / "Calculus.lean", *CALCULUS.rglob("*.lean")]):
        for line in path.read_text().splitlines():
            if line.startswith("import ") and not line.split()[1].startswith("Calculus."):
                bad.append(f"{path.relative_to(PROOFS)}: {line}")
    return bad


def unclaimed() -> list[str]:
    """Theorems a mutant names that the axiom audit does not require: a
    rename would then pass the audit and be caught only here."""
    claims = set(re.findall(r"``([\w.]+)", (PROOFS / "Audit.lean").read_text()))
    named = {name for m in (*MUTANTS, *CORE_MUTANTS) for name in m.fails}
    return sorted(named - claims)


def bad_sites() -> list[str]:
    """Each mutant whose site does not occur exactly once in its kernel file.

    A static pre-flight (#92): calibration refuses such a mutant only after
    copying and building, so a drifted site used to cost a whole run.
    """
    bad = []
    for mutant in MUTANTS:
        source = KERNEL / "src" / mutant.file
        count = source.read_text().count(mutant.before) if source.is_file() else 0
        if count != 1:
            bad.append(f"FAIL {mutant.name}: mutation site does not match exactly once ({count} matches)")
    for core in CORE_MUTANTS:
        source = CALCULUS / core.file
        count = source.read_text().count(core.before) if source.is_file() else 0
        if count != 1:
            bad.append(f"FAIL {core.name}: mutation site does not match exactly once ({count} matches)")
    return bad


def selection(argv: list[str]) -> tuple[bool, set[str] | None]:
    """`--check-sites` (only the pre-flight), `--only a,b` (those mutants;
    empty means none, as `cargo tauten proofs --level ship` asks when every
    claim's inputs are unchanged), or names as arguments; nothing selects all.
    """
    if argv == ["--check-sites"]:
        return True, set()
    if argv[:1] == ["--only"]:
        if len(argv) != 2:
            raise SystemExit("usage: mutants.py --only NAME[,NAME...]")
        return False, {n for n in argv[1].split(",") if n}
    return False, set(argv) or None


def main() -> None:
    if missing := unclaimed():
        raise SystemExit(f"FAIL: theorems named by mutants are not audited claims: {missing}")
    if bad := foreign_imports():
        raise SystemExit(f"FAIL: proofs/Calculus imports outside the calculus: {bad}")
    if missing := unnamed_laws():
        raise SystemExit(f"FAIL: calculus claims no core mutant names: {missing}")
    only_sites, selected = selection(sys.argv[1:])
    if bad := bad_sites():
        for line in bad:
            print(line, flush=True)
        raise SystemExit(f"FAIL: {len(bad)} mutant site(s) do not match exactly once")
    if only_sites:
        print(f"MUTANT_SITES ok ({len(MUTANTS) + len(CORE_MUTANTS)})", flush=True)
        return
    every = (*MUTANTS, *CORE_MUTANTS)
    mutants = [m for m in every if selected is None or m.name in selected]
    if selected and len(mutants) != len(selected):
        raise SystemExit(f"unknown mutant(s): {sorted(selected - {m.name for m in mutants})}")
    # Every mutant runs even after a failure, so one gate run reports them all.
    # Each calibrates in its own scratch copy, so they run side by side
    # (WRITFOLD_MUTANT_JOBS, default 6); the outcome does not depend on order.
    def one(mutant: Mutant | CoreMutant) -> str | None:
        with tempfile.TemporaryDirectory(prefix=f"mutant-{mutant.name}-") as directory:
            try:
                if isinstance(mutant, CoreMutant):
                    calibrate_core(mutant, Path(directory))
                else:
                    calibrate(mutant, Path(directory))
            except SystemExit as error:
                print(error, flush=True)
                return mutant.name
        return None

    with concurrent.futures.ThreadPoolExecutor(max_workers=JOBS) as pool:
        failed = [name for name in pool.map(one, mutants) if name is not None]
    if failed:
        raise SystemExit(f"FAIL: {len(failed)} mutant(s) not calibrated: {failed}")
    print(f"PROOF_CALIBRATIONS={len(mutants)}", flush=True)


if __name__ == "__main__":
    main()
