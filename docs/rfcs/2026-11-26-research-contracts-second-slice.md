---
rfc: "2026-11-26-research-contracts-second-slice"
title: "Research contracts, second slice"
status: accepted
implementation: not-started
authors:
  - tknawara
created: 2026-11-26
updated: 2026-11-26
issue: https://github.com/writfold-dev/writfold/issues/163
milestone: Later
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Research contracts, second slice: live reports, a binding that survives its step, a judged universe, unit-local staleness

Written from the review of the two landed epics
([#143](https://github.com/writfold-dev/writfold/issues/143), v0.6.0-dev.22;
[#153](https://github.com/writfold-dev/writfold/issues/153), v0.6.0-dev.23)
on 26 November 2026. It amends
[KB research contracts](2026-11-25-kb-research-contracts.md) and the
[agent vocabulary RFC](2026-11-25-agent-vocabulary.md) and supersedes
neither. Every case under [Motivation](#motivation) was reproduced against
the binary built from `main` at 1ae483f7, with the definitions of
`fixtures/kb/kb-consistency/`.

## Summary

The kernels of both epics are proved, and each is a fold over facts the
caller classified. The review found no fault in a kernel. It found faults
in the classification: a fact that is missing, a fact judged against "now"
where a record should decide, and one decision that is made in IO code and
handed to the kernel as a boolean.

Five decisions:

1. **A withdrawn report grounds nothing.** `ground::of_delivery` gains a
   `live` fact.
2. **A binding survives its step.** `invocation.bound` judges the step from
   what was recorded when the report was prepared, not from the step that
   is current when the check runs.
3. **The universe is a kernel decision.** A new `units::universe` decides
   whether a generated checklist has units, and it reads what the manifest
   says: its schema, its `coverage`, and whether each member's path is
   inside the scopes that are supposed to notice a change to it.
4. **Staleness is per unit.** A row for the pair `a~b` is about the
   definition, the seed, `a` and `b`. Another member's edit does not move
   it.
5. **`agent prepare` refuses two more things** (`--out` inside the KB, and
   `--for` an item that is not there) and keeps what each need stood on.

Nothing here adds a gate class, a record kind or a verb.

## Proof obligations

Listed first, per slice. Each slice's proofs gate (drift, `lake build`,
axiom audit, mutants) passes before its wiring.

### S1: `writfold_core::ground` (one function changed) and `spec/Invocation.tla`

- `of_delivery(found: bool, live: bool, ready: bool, bound: bool)
  -> Grounding`.
- `of_delivery_grounded_iff`, restated: grounded exactly when the candidate
  is found, live, ready and bound.
- `withdrawn_never_grounds`: with `live = false` the answer is
  `Ungrounded`, whatever the other three say.
- Mutant `delivery-withdrawn-grounds` (the `live` fact is not consulted)
  breaks both.

`invoke::bound` is unchanged; its `this_step` fact is classified
differently (below). The protocol gains one recorded fact, so the model
changes before the wiring, as for #145:

- `spec/Invocation.tla` gains the step a candidate was prepared at, and the
  invariant `CheckIsStable`: a candidate that passes `invocation.bound`
  still passes after `Advance`, while its record, its closure and its
  bytes are unchanged.
- A mutant cfg that judges `this_step` against the current step fails
  `CheckIsStable` and only that.
- The model's constants grow to two invocations of one step, so a retry
  is in it.

### S2: `writfold_core::units::universe` (new)

```text
universe(facts: &Scope) -> Result<(), Absent>
Scope  { defined, standing: Standing, reads, manifest, complete,
         inside, distinct, members: usize }
Absent { NoGate, NotChecked, Unreadable, NotManifest, Partial,
         OutsideScope, SameName, OverCap }
```

The first fact that does not hold, in that fixed order. `NotChecked` is
`ground::of_gate(defined, standing)` not being `Grounded`, so the rule for
"which standing of a gate counts as evidence" is stated once.

- `universe_ok_iff`: `Ok` exactly when every fact holds and
  `members ≤ MAX_MEMBERS`.
- `universe_first`: the refusal is the least fact, in the order above,
  that does not hold.
- `unchecked_scope_never_present`: a scope gate that is attested, waived,
  open, stale, failed or unknown gives no universe.
- `partial_manifest_never_present`: `complete = false` gives none,
  whatever the command's exit status was.
- `member_outside_scope_never_present`.
- `units::generated` takes `universe(..).is_ok()` as `present`;
  `absent_universe_never_covered` stands as proved.

| Mutant | Change | Breaks |
|---|---|---|
| `universe-attested-scope` | an attested scope gate gives a universe | `unchecked_scope_never_present` |
| `universe-partial-present` | `complete` is not consulted | `partial_manifest_never_present` |
| `universe-outside-scope-present` | `inside` is not consulted | `member_outside_scope_never_present` |
| `universe-over-cap-present` | the cap is not consulted | `universe_ok_iff` |

### S3: `writfold_core::units::unit_moved` (new)

- `unit_moved(definition_same: bool, seed_same: bool, first_same: bool,
  second_same: bool) -> Option<Moved>`, with
  `Moved { Definition, Seed, First, Second }`: the first that moved, in
  that order.
- `unit_current_iff`: `None` exactly when all four are the same.
- `unit_first_moved`: the least in the fixed order.
- Mutants `unit-ignores-second`, `unit-ignores-seed` and
  `unit-ignores-definition` each break `unit_current_iff`.

That no other member's bytes can move a row is true by the signature: the
function is not given them. The claim that this is the right signature is
an argument, under [Design](#unit-local-staleness), not a theorem.

### S4: `writfold_core::invoke::prepare_check` (two facts added)

- `Prepare` gains `out_outside: bool` and `for_item: ForItem`
  (`Absent`, `Known`, `Unknown`), judged after the six existing facts, in
  that order. `PrepareRefusal` gains `OutInsideKb` and `ForUnknown`.
- `prepare_check_spec`, `prepare_ok_iff` and the model's `firstRefusal`
  are restated. No existing refusal changes its place.
- `out_inside_refused`, `for_unknown_refused`.
- `prepare_spec`, `prepare_needs_ok_iff`, `needs_first` and
  `no_needs_unchanged` stand: `prepare` still wraps `prepare_check`.
- Mutants `prepare-skips-out` and `prepare-skips-for`.

### Tested, not proved

Reading withdrawals; recording and reading a candidate's step; reading the
manifest's `schema`, `coverage`, seed and member digests; matching a
member's path against scope patterns; naming units and finding two with
one name; resolving `--for` against the items a checklist has now; whether
`--out` resolves inside the KB root; every diagnostic. Each has a test
that fails when its check is removed, listed under
[Evidence and tests](#evidence-and-tests).

## Motivation

### A gate that reads met on a withdrawn report

```text
$ writfold kb gate cover pair-review 'm1~m2' supported delivery:CAND --by agent
$ writfold kb deliver withdraw CAND --reason superseded --by me
delivery CAND [withdrawn] …
$ writfold kb gate status
gate pair-review [attested] checklist 6/6 units covered
  item: m1~m2 [supported] 1 reference by agent
```

`gate/ground_delivery.rs` finds the candidate and never asks whether it
was withdrawn. `invocation.bound`'s own `single` fact does exclude
withdrawn candidates, so two parts of the product disagree about whether
the report exists.

### A good report that fails when it is checked again

```text
$ writfold kb plan advance        # review [done]
$ writfold kb deliver check CAND  # nothing edited
  check: invocation.bound [failed] … another-step (the invocation was
  prepared at kb-consistency#1/review, not at the step that owes this mode)
$ writfold kb gate status
gate pair-review [stale] checklist 5/6 units covered
```

`deliver/bound.rs` takes "the step that owes this mode" to be the run's
current step. Once the plan moves on, no report of the finished step can
pass a fresh check, the newest receipt decides, and the cover row that
cites it goes stale. The finding contradicts itself. A receiver that
re-executes checks, which is the received tier's whole method, would fail
every report of a finished run.

### A universe nobody judged

`gate/checklist/generated.rs` decides by early returns that the scope gate
must stand `Checked(Satisfied)`, that the kept file reads and that the cap
holds. The kernel receives `present: bool`. Three things written into the
manifest are never read back:

- **`coverage` and `schema`.** A kept file that says `partial` supplies a
  universe when its command exited 0.
- **Each member's `path` and `digest`.** A scope is current only because
  the gate's `scope` globs cover its members. The guide's example narrows
  the globs to four directories. A member under any other path can change
  and the receipt and every row stay current.
- **What the walk read.** A file added outside the globs that references
  the seed changes what the walk reaches (`others` went from 0 to 2) and
  the receipt still reads `[satisfied]`.

Unit names `<id>~<id>` are distinct only while no member id holds `~`.

### A review that its own fix invalidates

Ten members, 45 units covered. One sentence added to one member, the scope
gate run again:

```text
gate pair-review [stale] checklist 0/45 units covered
  item: m7~m8 [stale] … the scope's members or the definition changed since this row
```

Thirty-six of those reviews compare two records that did not change. The
`dispose` step exists to fix what the reviews found, and a fix is an edit
to a member. So the first disposition makes every review stale, and the
disposition's own attestation with it. The research RFC took "every
earlier row is stale" as the safe first rule and deferred reuse to #162 as
an optimization. It is the difference between a contract that converges
and one that cannot.

### Two refusals `agent prepare` does not make

`--out` inside the KB is accepted, and the guide says it never is. The
package then sits in the work tree, where the index reads it and a `**`
scope goes stale on it. `--for no-such-gate.x` is accepted, and the worker
is started on work no row can cite.

## Behaviour

### S1

A row that cites a withdrawn report is stale:

```text
  item: m1~m2 [stale] 1 reference by agent; cited evidence moved: delivery:CAND was ready, now withdrawn
```

`gate cover` refuses to cite one (`is not grounded (withdrawn)`).

`deliver prepare --invocation` writes the run's current step into the
candidate's record as `step`. `invocation.bound` then holds `this_step`
when that step is the invocation's step and that step owes
`deliver:<mode>`. `deliver check` gives the same answer before and after
`plan advance`. A report prepared while another step was current still
fails as `another-step`, now with a finding that names both steps.

### S2

A `why:` names the first fact that does not hold:

```text
gate pair-review [open] checklist @.writfold/gates/pair-review.toml:1
  why: related-scope's manifest lists inventory/assets/harbor-api.md, which is outside related-scope's scope; a change to it would not make the scope stale
```

`kb lint` reports an `items_from` whose gate is not defined, is not
mechanical or does not `retain` the file (`ref.undefined`,
`ref.wrong-kind`), and advises (never blocks) when a scope gate's `scope`
is narrower than the KB's records.

### S3

```text
gate pair-review [stale] checklist 36/45 units covered
  item: decision-m1~decision-m2 [stale] … decision-m1 changed since this row
```

A member added leaves earlier units covered and owes the new ones. The
seed edited makes every unit stale.

### S4

```text
refused: --out pkg is inside the KB; next: name a directory outside /path/to/kb
refused: --for pair-review.nope~nada: pair-review has no such unit now; next: writfold kb gate status pair-review
```

Exit 3, nothing written. An invocation record gains `needs`, one entry per
need with the gate id and the receipt or statement it stood on.

### What an existing setup can notice

A new optional key `step` in a candidate's record; `seed` (path and
digest) in `writfold-manifest/1`; unit digests in a generated checklist's
cover rows; `needs` in `writfold-invocation/1`; two new refusals at
`agent prepare`; a cover row citing a withdrawn report reads stale; a
manifest that says `partial` or lists a member outside a scope gives no
units.

## Design

### The step, from records

The evidence RFC already says a report "is judged against that record, not
against whichever run is current when it arrives". The same holds for the
step. The candidate's `step` is written under the write lock with the
shipped `Decided::Current` guard, as `agent prepare` writes its own. A
candidate without the key (written before this slice) is judged as today.

### What a scope's currency rests on

Deviation 1 of the research RFC (with `--manifest`, an edge declared under
`runs/` is not walked) stays. It has a consequence that was not written
down: the proved `complete_within` is a statement about the filtered edge
relation, and the boundary `runs/` now appears twice, once as a hard-coded
prefix in the walk and once as a pattern each gate author writes in
`scope`. `inside` joins them where it can be checked: a member the scope
would not notice changing is refused. What cannot be checked from a
manifest is a record that does not exist yet. Only `scope = ["**",
"!runs/**"]` makes "a newly related record cannot leave an old complete
standing" true. A narrower scope is the author's claim, and lint says so.

### Unit-local staleness

The claim a pair review makes is "these two records agree with each
other, as far as the reviewer's instructions ask". It depends on four
things: the two records, the instructions, and the seed that says why
these two were put side by side. The instructions are judged per report
by `invocation.bound` (`closure_current`). The other three are the facts
of `unit_moved`, with the checklist's definition.

A third member cannot change what the pair means, because the reviewer
was handed only the pair: `agent prepare --for` records the input's digest
and the fixture's input is the unit's name. A profile whose instructions
tell the reviewer to read the whole scope makes a different claim, one
this rule does not cover. That is a different unit rule (#162), never a
reading of `pairs`.

A row records the three digests it is about. The manifest's whole digest
stops joining a row's digest, which is deviation 3's `rows_digest`. The
definition digest stays static, so the run's pin holds, as today.

### The universe must still be present

S3 does not let a row stand without a scope. When the scope gate is stale
there are no units (S2). Run again, the units return and each row is
compared with the new manifest's digests.

## Compatibility and reversibility

Every new key is optional. A record without one is judged by today's
rule, and in each case today's rule is the stricter: a candidate without
`step` against the current step, a row without unit digests against the
whole manifest. Nothing that is stale today becomes current without a new
row or a new check. Two things that read met today stop doing so: a row
on a withdrawn report, and a checklist whose manifest says `partial` or
lists a member outside a scope. Both are corrections.

## Alternatives

| Alternative | Why it lost |
|---|---|
| Leave `this_step` as it is and tell users not to re-check | A check whose answer depends on when it is run is not evidence, and a receiver re-runs checks |
| Hold `this_step` when the invocation's step is current or done | A loop round's old report would pass in a later round. A recorded step needs no such exception |
| Compare every member's digest with present bytes on each read, and drop the scope globs | Two notions of "the scope moved". The globs also catch a record that is new, which no manifest can |
| Keep "everything stale" and make reviews cheap to redo | It costs a model call per unit, and the disposition step still cannot end current |
| A joined unit digest judged by the shipped `gate::about` | No new kernel, but the `why:` could not name which record moved |
| Validate `--for` only at `gate cover`, as now | The mistake costs a whole worker run before anything says so |

## Evidence and tests

- `kb-consistency-rehearsal.sh` gains sections, each asserted in
  `kb_consistency.rs`: a report withdrawn after it covered a unit; a
  report checked again after its step is done; a manifest that says
  `partial` with exit 0; a member outside the scope; one member edited
  (the owed units named); the seed edited; `--out` inside the KB; `--for`
  an unknown unit.
- A property test over random edits, additions and removals of members:
  exactly the units naming an edited member go stale.
- The untested behaviour #168 lists for #156 (`items` with `items_from`,
  neither, `empty = "allowed"` with zero and one member, an unreadable
  manifest).
- TLC over `spec/Invocation.tla` with the new invariant and mutant cfg.
- The proofs gate with the ten mutants above; quality and unit.

## Rollout

| Step | Issue | Ships |
|---|---|---|
| S1 | [#164](https://github.com/writfold-dev/writfold/issues/164) | `live`; the candidate's `step`; the model first |
| S2 | [#165](https://github.com/writfold-dev/writfold/issues/165) | `units::universe`; the lint codes and the advice |
| S3 | [#166](https://github.com/writfold-dev/writfold/issues/166) | `units::unit_moved`; the seed in the manifest; unit digests in rows |
| S4 | [#167](https://github.com/writfold-dev/writfold/issues/167) | the two refusals; `needs` in the record |
| S5 | [#168](https://github.com/writfold-dev/writfold/issues/168) | the tests the two epics left out; corrected text |

S1 first: two standings read wrong today. S2 before S3, because a row may
only outlive its neighbours while the universe is judged. S5 at any time.

## Unresolved questions

None blocks acceptance. The owner questions are in
[#160](https://github.com/writfold-dev/writfold/issues/160): whether the
seed is a member of its own scope; whether KB documents that are not
records are members; whether the seed's bytes belong to a unit.

## Decision log

- 2026-11-26: Written from the review of #143 and #153 after both landed.
  Accepted under the standing rule (owner, 21 November 2026: RFCs are
  accepted by default; the owner reviews and amends afterwards).
  Implementation is not started and is the main session's. The defaults
  taken: the seed's bytes are part of every unit; the seed is not a
  member; KB documents that are not records stay in `others`.
