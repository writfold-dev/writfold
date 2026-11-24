---
rfc: "2026-11-24-resume-context"
title: "The handover packet: a bounded resume whose required rows cannot be silently dropped"
status: accepted
implementation: complete
authors:
  - tknawara
  - opus
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/109"
milestone: "v0.6.0-dev.19"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: The handover packet

Adopted from the draft in PR #114 and trimmed to its first slice in the
dev.19 grooming (#126). Snapshot retry, the session-start transport (#123),
lessons (#108) and plan revision (#110) are later slices.

## Proof obligations

A new kernel, `writfold_core::packet`, holds three functions:
- `cut(costs, budget)`: how many leading rows fit;
- `required_omitted(kept, required)`: how many required rows a packet of
  `kept` rows left out;
- `complete(kept, required, unreadable)`: whether the packet may say it is
  complete.

`proofs/Writfold/Packet.lean` proves:

- `cut_spec`: the cut `k` is at most the number of rows. Its parts:
  - `select_within_budget`: the first `k` rows cost at most the budget.
  - `cut_maximal`: when a row is left out, the next one did not fit.
- `all_fit_all_selected`: if every row fits, every row is kept.
- `required_omitted_spec` and `omitted_counted`: the number of required rows
  reported as left out is `required - min kept required`. It is zero exactly
  when every required row was kept.
- `complete_iff`: the packet is complete exactly when every required row is
  kept and nothing it was built from was unreadable.

Two of the draft's properties hold by construction. A prefix keeps the
input order (`select_keeps_order`), and the caller lists required rows
first, so no optional row is kept while a required one is dropped
(`required_before_optional`).

The draft's `context_no_status_promotion` and `context_provenance_preserved`
are properties of how rows are gathered, not of a selector over costs. The
packet reuses `resume`'s rows unchanged, with their standings, sources and
flags, and that is tested.

Mutants:
- `cut-ignores-used` breaks `cut_spec` and `select_within_budget`;
- `cut-drops-exact-fit` breaks `cut_spec` and `all_fit_all_selected`;
- `omitted-reads-zero` breaks `required_omitted_spec` and `omitted_counted`;
- `packet-complete-ignores-unreadable` breaks `complete_iff`.

**Tested, not proved:**
- which rows are required;
- each row's cost (the pager's own token estimate, `emit::row_cost`);
- gathering the rows;
- rendering.

## Behaviour

`writfold kb resume [<run>] --packet [--budget-tokens N] [--json]` gives
the rows of `resume`, reordered and cut to the budget.

**Required rows, in `resume`'s order:**
- the run (task, state, worktree, branch);
- the plan position with its `next:` line;
- gates not met, and deliveries not ready;
- unconfirmed drafts (an interrupted attempt);
- the newest checkpoint;
- any row that did not read (`invalid`, `unreadable`, `unverifiable`).

**Optional rows, after them in `resume`'s order:** gates already met,
coverage and recheck rows, artifacts and pending proposals.

The packet is the longest prefix within the budget. The trailer adds
`required=N required_omitted=N optional_omitted=N packet=complete|partial`.

- A required row that does not fit makes the answer
  `answer=unknown reason=budget`, exit 2. A note names
  `writfold kb resume <run>`, which shows every row, paged with `--cursor`.
- An input that did not read makes the packet `partial`. It is never
  reported complete.
- Dropped optional rows are counted and named in a note.

Nothing is written, and no gate is run. Every row keeps its source path and
line and its own standing.

## Alternatives

- **Separate queries** (`resume`, `plan status`, `gate status`). This stays
  available and is the baseline. The packet adds only the
  required/optional split and the completeness verdict.
- **A greedy selector that skips an oversized optional row and keeps
  later ones.** This was rejected for now. Its proof is harder and the
  benefit is unclear. Optional rows are ordered by priority instead.
- **A new `handover` verb.** This was rejected because it reads too close to
  `handoff check`, which is the receiver's verb.

## Limits

- The packet reads the KB once and names its pin in the trailer. It does
  not retry when a record changes while it is being assembled.
- Costs are estimates (about 4 bytes per token), the same estimate the
  pager uses everywhere.

## Acceptance gates

| Gate | Evidence |
|---|---|
| proofs | `Writfold.Packet` builds; the 4 mutants calibrate |
| unit | `crates/writfold-kb/tests/packet.rs`: required rows first, a roomy packet complete, a tight budget drops only optional rows, a tiny budget is `unknown reason=budget` with exit 2 |

## Decision log

- 2026-11-24: adopted from PR #114, trimmed (#126 grooming) and accepted by
  default under the standing acceptance (AGENTS.md). Implemented in
  v0.6.0-dev.19.
