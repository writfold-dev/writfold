---
rfc: "2026-11-26-pinned-agent-closures"
title: "Pinned agent closures: an edited profile reads policy moved"
status: accepted
implementation: complete
authors:
  - tknawara
created: 2026-11-26
updated: 2026-11-26
issue: https://github.com/writfold-dev/writfold/issues/161
milestone: v0.6.0-dev.25
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Pinned agent closures: an edited profile reads policy moved

Amends the [agent vocabulary RFC](2026-11-25-agent-vocabulary.md) (its
scope line "the definitions a run's modes name join the run's policy pins,
so an edited profile reads `policy moved`") and the
[evidence-governed RFC](2026-11-25-evidence-governed-execution.md) (its
Trust row). Supersedes neither.

## Summary

A run's policy pins hold each gate word its spliced steps owe, with that
definition's digest. A delivery word `deliver:<mode>` is one of them: the
pin holds the mode file's digest and the digests of the gates it names,
and while the step that owes it is open, an edited mode reads `policy
moved` ([`plan/drift.rs`](../../crates/writfold-kb/src/plan/drift.rs)).
#161 said modes are not pinned at all; that is not so, and this RFC adds a
test that pins it.

What is not pinned is the **agent** a mode names (`agent = "<id>"`). Its
closure (the profile, its skills, its tools and their instructions) is
checked per report by `invocation.bound`, which compares the closure an
invocation recorded with the closure now. It is not in the run's policy:
an edited profile stales a report, and the run's summary never says
`policy moved`.

One decision: **a pin holds, for each agent a pinned mode names, the
closure digest of that agent**, under the entry name `agent:<id>`. It is
compared exactly as a delivery word is: while a step owing a mode that
names the agent is open, against the closure now; once that step is done,
not at all (the done step's own `!stale`, from `invocation.bound`'s
re-check, covers it). `plan repin` adopts it like any other entry.

Nothing new is decided: the comparison is `policy::pin_one` and the run's
answer `policy::pin_all`, whose theorems hold for any entry.

## Proof obligations

No kernel function changes. The obligations are the existing theorems,
stated for the new entries, and one composed theorem that makes the claim
"an edited closure reads moved" a statement in Lean rather than an
argument.

- `Policy.pin_one_spec` and `pin_one_same_iff`: an entry is a pair of
  optional byte strings, whatever it names. An `agent:<id>` entry whose
  pinned digest differs from the closure now is `Moved`.
- `Policy.pin_all_spec`: the run reads `Moved` exactly when some entry is
  `Moved`.
- New, `Policy.pinned_change_is_moved`: an entry pinned with some bytes is
  `Moved` when the present has other bytes or nothing loads, whatever it
  names. With the existing `Policy.pin_all_moved_iff` (one `Moved` entry
  makes the run `Moved`), it is what the wiring relies on.
- Existing mutants `pin-one-missing-same`, `pin-all-misses-moved` break
  these. The new entries add no kernel mutant. Their wiring is tested
  instead: removing the `agent:` entry from the pin, or skipping it in the
  drift, fails a test (below).

The receiver (`handoff check`) gains no fact. Its required set is gates
at the operator's `--policy` revision, each met only by a mechanical
receipt judged under the definition at that revision. It reads no
delivery mode and no agent definition, so a mode or profile edited in the
candidate cannot change what it accepts. This RFC states that and tests
it; it does not add modes to `handoff`.

`spec/RunGraph.tla` already models a template edited mid-run, and the pin
write is the existing one: no model change.

## Behaviour

```text
$ writfold kb plan status
…
# note: policy moved since this run pinned it: 1 gate (agent:reviewer); the plan cannot read complete until the definitions are restored or the run adopts them: writfold kb plan repin --reason TEXT --by WHO
```

The step owing `deliver:review` shows why:

```text
step claims-review#1/review [current] owes: deliver:review [open] … !moved
  why: agent:reviewer is now @1a2b3c4d5e6f; this run pinned agent:reviewer@9f8e7d6c5b4a: evidence is judged under the definition as it is now
```

`plan repin --reason TEXT --by WHO` records the closure as it is now; the
plan then ends at best `complete-with-exceptions`, as for a gate.

### What an existing setup can notice

A pin written from v0.6.0-dev.25 on holds `agent:<id>` entries for each
agent a spliced mode names. A run pinned earlier has none, and is judged
as before. An open step owing a mode whose agent's closure changed since
the pin makes the run read `policy moved`.

## Design

- **Where the closure comes from.** The digest `agent inspect` prints and
  the invocation record holds (`inspect::decide` over the KB root and the
  run's repository). A closure that does not resolve (a missing skill)
  counts as "no longer loads", like a gate that no longer loads.
- **Which agents.** Those named by a mode a spliced step owes, once each,
  in the pin that first spliced a step owing such a mode. A repin holds
  every agent of the plan's modes, as it holds every gate.
- **Why not join the closure into the mode's own entry.** Every run pinned
  before this change has a mode entry without it, and would read `policy
  moved` the day it upgraded. A separate entry keeps old runs as they were.
- **Not pinned: `items_from`.** A generated checklist's scope gate is
  pinned as a gate already. Its manifest is evidence, not policy, and is
  judged by the universe (#165).

## Compatibility and reversibility

The entry is new and optional. A pin without `agent:` entries reads as
before. Removing the feature leaves pins with entries that no drift reads,
which is harmless.

## Alternatives

| Alternative | Why it lost |
|---|---|
| Append the closure to the mode's entry | Every run pinned before reads `policy moved` on upgrade |
| Compare agent entries on done steps too | A gate on a done step is compared with what it was met under, not with now; the done delivery step has its own `!stale`, which `plan status` shows since #168 |
| Add modes to `handoff check` | The receiver accepts only mechanical receipts of gates at its own revision; a mode is never one, so nothing there can be fooled by an edited mode |

## Evidence and tests

- `plan_repin.rs` or a new `agent_pins.rs`: an edited mode reads `policy
  moved` while its step is open (the case #161 asked for); an edited
  profile, skill, tool or instructions file of a mode's agent reads
  `policy moved`; `plan repin` adopts it and the plan ends at best
  `complete-with-exceptions`; a run pinned before (no `agent:` entries) is
  judged as before; a mode no spliced step owes adds no entry.
- `handoff.rs`: a candidate whose mode or profile was edited gets the same
  verdict as one whose was not.
- The proofs gate with `pinned_change_is_moved`; quality and unit.

## Unresolved questions

None blocks acceptance.

## Decision log

- 2026-11-26: Written for #161 from reading `plan/drift.rs` and
  `plan/pin_write.rs`: modes are pinned already (open steps), and the gap
  is the agent's closure. Accepted under the standing rule (owner, 21
  November 2026). The defaults taken: a separate `agent:<id>` entry; done
  steps not compared; `handoff check` unchanged and tested.
- 2026-11-26: Built in v0.6.0-dev.25. As built: the entries are written by
  the existing pin write and read by the existing drift and repin, through
  `plan/agent_pins.rs`; `Policy.pinned_change_is_moved` is the new theorem
  (with the existing `pin_all_moved_iff`). A tool file has only a
  `description`, which is not behaviour, so an edited tool file moves
  nothing; the tests edit the profile, a skill and the `model` hint.
