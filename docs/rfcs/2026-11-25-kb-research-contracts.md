---
rfc: "2026-11-25-kb-research-contracts"
title: "KB research contracts and review coverage"
status: accepted
implementation: not-started
authors:
  - tknawara
created: 2026-11-25
updated: 2026-11-25
issue: https://github.com/writfold-dev/writfold/issues/155
milestone: v0.6.0-dev.23
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-25-agent-vocabulary"
  - "https://github.com/writfold-dev/writfold/issues/149"
---

# RFC: KB research contracts and review coverage

Groomed on 25 November 2026 with the
[agent vocabulary RFC](2026-11-25-agent-vocabulary.md) and
[evidence-governed execution](2026-11-25-evidence-governed-execution.md).
The first draft proposed six new kernels, a `derived` gate class and a
scope-recipe schema. The
[grooming review](2026-11-25-agent-vocabulary/grooming.md) (rows R1 to
R11) found that a scope is a mechanical gate with retained evidence, that
review units are checklist items, and that findings are an attested gate.
What is new is small: a manifest, generated items, and a report as the
evidence of a row. The shared algebra is in the
[calculus](2026-11-25-agent-vocabulary/calculus.md#39-coverage-of-a-generated-universe).

This is the second milestone's work. It is written down now so that the
first slice does not close a door on it.

## Summary

"Check all related KB records for inconsistencies" mixes three things: a
scope, the work of reviewing it, and a judgment. Each becomes a gate on
its own plan step.

| Step | Gate | Class | What it says |
|---|---|---|---|
| enumerate | a scope gate | mechanical, as shipped | the related records were read in full, and here is the list |
| inspect | a unit checklist | checklist, with generated items | every pair on that list has a review |
| reconcile | a disposition gate | attested, as shipped | someone dealt with what the reviews found |

What each review says is in its report. It reaches no standing. So four
things are shown and never folded into one word: the scope, the coverage,
the assessments and the disposition.

New: `kb impact --manifest FILE`; three checklist keys (`items_from`,
`units`, `allow_empty`); a `delivery:<id>` reference in a cover row; and
`agent prepare --for GATE.ITEM`.

## Proof obligations

### `writfold_core::units` (new)

- `pairs(n: usize) -> Option<Vec<Pair>>`: every `(a, b)` with
  `a < b < n`, in lexicographic order, or `None` when `n` is over
  `MAX_MEMBERS` (64, so at most 2,016 units).
- `generated(present: bool, count: usize, allow_empty: bool,
  rollup: Rollup) -> Rollup`: what a checklist with generated items adds
  up to.

`proofs/Writfold/Units.lean` proves:

- `pairs_spec`: the list is exactly the pairs `a < b < n`, in order, each
  once.
- `pairs_len`: it has `n(n−1)/2` entries.
- `over_cap_refused`: `None` exactly when `n > MAX_MEMBERS`. A work limit
  never produces a shorter list that could read as complete.
- `generated_covered_iff`: covered exactly when the universe is present,
  it has at least one unit or the checklist allows an empty one, and the
  shipped roll-up says every unit is covered.
- `absent_universe_never_covered`: with no manifest, the checklist is not
  covered, whatever rows exist.
- `empty_universe_needs_allow`: zero units are covered only when the
  checklist says `allow_empty = true`.

The guard matters because the shipped roll-up over zero items is true:
"every required item is covered" holds of none.

### `writfold_core::ground` (one function added)

- `of_delivery(found: bool, ready: bool, bound: bool) -> Grounding`: a
  cover row may cite a report.
- `of_delivery_grounded_iff`: grounded exactly when the candidate exists,
  is ready and passes `invocation.bound`.

Which item the report is for is the shipped `bind` kernel's question. A
report whose invocation was prepared `--for` this item is `Bound::This`;
one prepared for another item is `Bound::Other` and answers nothing
(`other_never_answers`).

### Mutants

| Mutant | Change | Breaks |
|---|---|---|
| `pairs-drops-last` | the inner loop stops one short | `pairs_spec` |
| `pairs-keeps-diagonal` | `(a, a)` is emitted | `pairs_spec` |
| `generated-absent-covered` | an absent universe returns the roll-up | `generated_covered_iff` |
| `generated-empty-covered` | zero units are covered without `allow_empty` | `generated_covered_iff` |
| `delivery-unready-grounds` | an unready report grounds a row | `of_delivery_grounded_iff` |

### Reused as they stand

- The scope is complete: `complete_within_iff`,
  `truncated_never_complete`; a timeout, a signal or a moved subject is
  unknown (`outcome_pass_iff`); a missing manifest file is never a pass
  (`kept_missing_never_pass`).
- The scope is still about the present KB: `judge_other_bytes`,
  `state_spec`.
- One unit: `item_spec`, `item_under_spec`, `item_under_narrows`.
- All units: `rollup_covered_iff`, `standing_never_satisfied`.
- The disposition: `standing_attested_iff`, `waiver_rejected_never_met`.

### Tested, not proved

Writing and reading the manifest; mapping members to unit names; the
digest that joins a generated checklist to its manifest; resolving a
`delivery:` reference; rendering. A proof of coverage is about the
declared list. It is not a claim that every relevant record is on it, or
that a review is right.

## Motivation

The shipped `research.backfilled` check shows that something was written
during a step. It does not show that every related record was looked at.
An agent can run one search, read three of five records and write a
summary. Nothing owed says which two were skipped.

## Behaviour

### The scope gate

```toml
# .writfold/gates/related-scope.toml
schema = "writfold-gate/1"
id = "related-scope"
version = 1
class = "mechanical"
argv = ["writfold", "kb", "impact", "harbor-api", "--direction", "both",
        "--depth", "2", "--manifest", "{artifacts}/manifest.json"]
workdir = "."
timeout_seconds = 60
scope = ["**", "!runs/**"]
retain = ["manifest.json"]
accept = ["satisfied"]
description = "every record within two links of harbor-api, read in full"
```

`kb impact` already walks typed links from a record, bounded, and answers
`coverage=partial` with exit 2 when it stopped early. `--manifest FILE`
writes what it reached:

```json
{
  "schema": "writfold-manifest/1",
  "seed": "harbor-api",
  "direction": "both",
  "depth": 2,
  "coverage": "complete",
  "members": [
    {"id": "adr-12", "path": "decisions/adr-12.md", "digest": "sha256:…"},
    {"id": "adr-31", "path": "decisions/adr-31.md", "digest": "sha256:…"}
  ]
}
```

Members are sorted by id. The gate machinery does the rest:

- A partial walk exits 2, so the receipt is `fail`.
- A timeout or crash is `unknown`.
- The manifest is kept with the receipt and checked on every read.
- The `scope` covers the KB's records. When a record is added, removed or
  edited the scope digest moves and the receipt is `stale`. A newly
  related record cannot leave an old "complete" standing.

"Complete" means complete for this seed, direction and depth over typed
links. It is not a claim about records that are related in meaning and
not linked. Search hits can suggest links to record; they do not widen the
scope by themselves.

### The unit checklist

```toml
# .writfold/gates/pair-review.toml
schema = "writfold-gate/1"
id = "pair-review"
version = 1
class = "checklist"
items_from = "related-scope/manifest.json"
units = "pairs"
allow_empty = false
scope = ["**", "!runs/**"]
description = "every pair of related records compared"
```

The items are not listed in the file. They are the pairs of the members
of the manifest retained by `related-scope`'s deciding receipt, named
`<id>~<id>` with the ids in byte order. When that receipt is not a
current pass, or its manifest does not read, there are no items and the
checklist is not covered:

```text
gate pair-review [open] checklist @.writfold/gates/pair-review.toml:1
  why: its items come from related-scope, which is [stale]; next: writfold kb gate run related-scope
```

Each unit is covered by a row that cites a report:

```sh
writfold kb agent prepare kb-reviewer --for 'pair-review.adr-12~adr-31' \
  --input pair.json --out /tmp/inv
# the worker writes report.md
writfold kb deliver prepare --mode pair-report --from report.md --invocation ID
writfold kb deliver check
writfold kb gate cover pair-review 'adr-12~adr-31' supported delivery:CANDIDATE --by WHO
```

`--from` rows cover many units in one write, as today. With eight of ten
units covered:

```text
gate pair-review [open] checklist 8/10 units @.writfold/gates/pair-review.toml:1
  item: adr-12~adr-40 [open]
  item: adr-31~adr-44 [open]
  next: writfold kb gate cover pair-review 'adr-12~adr-40' supported delivery:ID --by WHO
```

A row says a review exists for the unit. Whether the review found a
conflict is in the report. A unit whose review could not be done is
covered `unsupported` with a reason, and stays owed.

### The disposition gate and the plan

```toml
# .writfold/gates/findings-dispositioned.toml
schema = "writfold-gate/1"
id = "findings-dispositioned"
version = 1
class = "attested"
scope = ["**", "!runs/**"]
accept = ["attested"]
instructions = "findings-dispositioned.md"
description = "each conflict the reviews found is resolved, kept open with a reason, or excepted by name"
```

```toml
# .writfold/plans/kb-consistency.toml
schema = "writfold-plan/1"
name = "kb-consistency"
steps = [
  "enumerate: related-scope",
  "inspect: pair-review",
  "reconcile: findings-dispositioned",
]
```

The three steps are the contract. There is no fourth gate that derives a
verdict from the other three. A research task may end with open findings
if the instructions allow it; a publishing step can owe a stricter gate.

### When the KB moves

Add a sixth record that links to the seed. Then:

1. `related-scope` reads `stale`. The `enumerate` step shows `!stale` and
   stays done.
2. `pair-review` has no items until `related-scope` is run again. Then it
   has fifteen.
3. Every earlier row is stale, because a row is about the checklist's
   definition and the manifest it was generated from.

Step 3 is the safe first rule, and the draft's own starting point ("pin
the whole relevant corpus"). The dry runs reused the ten earlier reviews.
That needs a digest per unit and an argument that a new member cannot
change what an old pair means. It is later work, noted in #156.

### What an existing setup can notice

New: the `--manifest` flag; `items_from`, `units` and `allow_empty` on a
checklist; `delivery:<id>` as a cover reference; `--for` on
`agent prepare`. A checklist with neither `items` nor `items_from`, or
with both, is refused.

## Design notes

- **Unit names** use record ids, not versions. A changed member makes its
  rows stale. It does not mint new units and orphan old ones.
- **The cap.** Over 64 members the checklist is refused with the count. A
  rule that partitions by subject is a different coverage claim and a
  later key, never a silent substitute.
- **No new protocol.** Every write here is an existing publication:
  a gate receipt with retained evidence, a cover row, a delivery. No TLA+
  model changes.
- **No retrieval receipt.** The manifest is a gate's retained file. The
  retrieval receipts removed in #105 do not return.

## Points to settle when the slice starts

- Whether a read verb may run as a gate's command while `gate run` holds
  the run (expected: yes, `impact` takes no write lock). If not, the scope
  gate becomes a built-in `run-record` requirement instead.
- One seed per manifest, as `impact` takes one id today, or several.
- Whether `gate cover --from` should take a manifest of reports, so a
  driver covers all units in one call.

## Compatibility and reversibility

All keys are new and optional. A checklist with listed items is
unchanged. Removing the feature is removing the three gate files.

## Alternatives

| Alternative | Why it lost |
|---|---|
| Six new kernels (scope eligibility, scope freshness, unit generation, report eligibility, coverage, disposition), as drafted | Four are shipped kernels under other names |
| A `derived` gate that requires named predicates of other gates | The removed assessed gate by another name; the plan already orders and conjoins |
| A scope-recipe schema with stores, relations and closure policy | The recipe is the scope gate's command; its bytes are in the definition digest |
| Keep old pair reviews when a member is added | Needs per-unit digests and its own argument; deferred |
| Let the agent list what it reviewed | That is today's behaviour, and the problem |

## Evidence and tests

`fixtures/kb/kb-consistency/`: records A to E linked to one seed, a fake
worker, and these cases:

- ten units; eight covered; the two owed are named; the last two covered;
  the step advances; the disposition gate is still owed;
- a walk stopped by a work limit: the scope gate fails;
- a search that times out: unknown;
- record F added: scope stale, no items, then fifteen, earlier rows stale;
- a report prepared for another unit: the row is refused;
- a schema-valid report with a wrong reading: every gate can be met. The
  fixture says so in a comment. Coverage is not correctness.

Plus the proofs gate with the five mutants, quality and unit.

## Rollout

| Step | Issue | Ships |
|---|---|---|
| 1 | [#155](https://github.com/writfold-dev/writfold/issues/155) | `impact --manifest`, the scope-gate recipe, tests for partial, unknown and membership change |
| 2 | [#156](https://github.com/writfold-dev/writfold/issues/156) | `units` proved; generated checklist items; `delivery:` references; `--for` |
| 3 | [#157](https://github.com/writfold-dev/writfold/issues/157) | the fixture and a guide page |

## Unresolved questions

None blocks acceptance.

- Owner: is "everything stale when the KB moves" acceptable as the first
  rule? Default: yes.

## Decision log

- 2026-11-25: Scope completeness, review coverage, assessment and finding
  disposition separated at the owner's request. A five-document, ten-unit
  fixture defined.
- 2026-11-25: Dry-run results added; collection-level invalidation
  distinguished from unit reuse.
- 2026-11-25: Groomed. Outcomes: the scope is a shipped mechanical gate
  with `retain`; units are generated checklist items; the disposition is a
  shipped attested gate; one new kernel (`units`) and one new grounding,
  seven theorems and five mutants; the `derived` class, the `[execution]`
  block and the scope-recipe schema removed; unit reuse deferred. Status
  left `draft` for the main session to accept.
- 2026-11-25: Accepted at the owner's request, taking the default that a
  KB move makes every earlier review row stale as the first rule.
  Implementation (#155, #156, #157) is the v0.6.0-dev.23 milestone, after
  the dev.22 agent vocabulary slice.
