---
rfc: "2026-11-27-claims-in-text"
title: "Claims inside text: a minimum number of citations, and every block of a draft accounted for"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/189
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Claims inside text

Law U of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md),
applied to the blocks of a draft.

## Proof obligations

**Kernel:** a new module, `writfold_core::claims`.

```rust
/// One prose block of a candidate, as the caller classed it.
pub enum Block {
    Cited(Standing),   // holds at least one citation token; how they stand together
    Tagged,            // carries exactly one status tag the mode takes
    Excused,           // a reviewer's row says it makes no claim (about these bytes)
    Bare,              // none of the above
}
pub enum Standing { Fresh, Stale, Unverifiable }

/// `claims.accounted`: every block is cited and fresh, tagged, or excused.
pub fn accounted(blocks: &[Block]) -> Outcome;      // Pass, Fail, Unknown

/// `claims.cited-fresh` with a minimum: the shipped outcome, never a
/// pass below `min` tokens.
pub fn cited(tokens: usize, min: usize, shipped: Outcome) -> Outcome;
```

`accounted`: no block is `Unknown` (a draft with nothing to account for
is never a pass); any `Bare` or `Cited(Stale)` block is `Fail`; otherwise
any `Cited(Unverifiable)` is `Unknown`; otherwise `Pass`. `cited`:
`tokens < min` is `Fail`; otherwise `shipped`.

**Theorems** (`proofs/Writfold/Claims.lean`):

- `accounted_spec`.
- `accounted_pass_iff`: `Pass` exactly when there is at least one block
  and every block is `Cited(Fresh)`, `Tagged` or `Excused`.
- `bare_never_passes`: one `Bare` block and the outcome is `Fail`.
- `empty_never_passes`.
- `remainder_is_bare`: with each block mapped to `account::Member`
  (`Bare` to `Remainder`, `Excused` to `DeclaredOut`, the rest to
  `Judged`), `Pass` implies `account::whole`.
- `tag_is_not_a_citation`: turning a `Tagged` block into `Cited(Fresh)`,
  or back, changes no outcome. A tag accounts for a block; it says
  nothing about its truth, and the receipt's note counts tags apart.
- `cited_spec`; `below_min_never_passes`; `no_min_unchanged` (`min = 0`
  returns `shipped`, so an existing mode notices nothing).

**Mutants:** `bare-passes`, `empty-passes`, `unverifiable-passes`
(`accounted_spec`); `min-ignored` (`cited_spec`).

**Tested and unproved:**

- Splitting a candidate into blocks, with the reader `prose.rules`
  already uses (`deliver/markdown.rs`): a paragraph, a list item and a
  table row are blocks; headings, code blocks, front matter, link
  reference definitions and the candidate's own trailer lines are not.
- Finding tokens in a block and judging them (the shipped
  `claims.cited-fresh` path).
- The status-tag grammar.
- Binding an `Excused` row to its block (below).

No protocol changes. No TLA+ model.

## Problem

`claims.cited-fresh` checks the citation tokens a draft has. "A draft
with no tokens passes, and the note says `0 citation tokens`"
(`delivery-modes.md:101`), and "a claim made in prose without a citation
token is not seen" (`:428`). So grounding is judged on the citations
present, not on the text: the fewer a draft cites, the easier it passes.

Writfold cannot tell which sentence is a claim. It does not have to. The
universe is the draft's blocks, which it can list, and each block is
either accounted for or left over.

## Decision

### `min_citations`

A mode key for `claims.cited-fresh`: `min_citations = 1`. Below it:
`failed[claims.cited-fresh]: 0 citation tokens; this mode takes at least
1 (min_citations)`. Without the key the check is as shipped.

### `claims.accounted`

A new built-in check, mechanical, never a default, listed by a mode that
wants it:

```toml
# .writfold/deliveries/design-note.toml
checks = ["claims.cited-fresh", "claims.accounted"]
status_tags = ["inferred", "recalled", "unknown"]
excuse = "claims-reviewed"          # optional: a checklist gate whose rows may excuse a block
```

Each block is classed:

| Class | When |
|---|---|
| cited | it holds a citation token; the tokens are judged as `claims.cited-fresh` judges them |
| tagged | it ends with one of the mode's status tags in square brackets: `… the cache is never flushed there [inferred]` |
| excused | the `excuse` gate has a current cover row for this block, `not-applicable`, with a reason |
| bare | none of these |

A bare block fails the check and is drawn at its line:

```text
failed[claims.accounted]: 3 of 14 blocks carry no citation and no status tag
  --> candidate.md:22
   |
22 | The retry budget is shared across tenants.
   | ^ cite it, tag it ([inferred], [recalled], [unknown]), or excuse it: writfold kb gate cover claims-reviewed b:5f93c1 not-applicable --reason TEXT
```

The receipt's note says what the pass rests on:
`14 blocks: 8 cited (11 tokens, fresh), 4 tagged (3 inferred, 1 unknown), 2 excused`.
A reader of a ready candidate sees how much of it is tagged rather than
cited; the check never reads "grounded".

**The excuse checklist.** With `excuse`, the named checklist gate takes
its items from the candidate (`items_from = "candidate"`,
`units = "blocks"`): one item per bare block, named by the block's digest
(`b:<12 hex>`), so editing one block stales only its row (the unit-local
rule of #166). A row is the reviewer's statement that the block makes no
checkable claim (a transition, a question, an instruction). It stands as
attested, and the mode's `waivable` and the gate's `accept` decide
whether that is enough, as for any checklist. Without `excuse`, a block
is cited or tagged or the check fails.

**The status-tag grammar.** A tag is one of the mode's words in square
brackets at the end of a block. A bracketed word at the end of a block
that is one edit from a tag (`[infered]`) is a finding with a suggestion,
never a silent bare block. Two tags on one block, or a tag on a block
that also cites, are findings: a block says one thing about its support.

### What this does not give

- **Not truth.** A cited block's citation is fresh; whether it supports
  the sentence is a reviewer's judgment (an attested gate), as today.
- **Not claim detection.** A block is the unit. A paragraph with one
  cited sentence and one uncited one is cited. A team that needs finer
  grain writes one claim per list item.
- **A subagent's claim** is not a class of its own. A block that rests
  on a worker's report cites the report (`delivery:<candidate>`), which
  is grounded only while the report is live and bound (#164), or is
  tagged `[recalled]`.

### What an existing setup can notice

- Nothing without the new keys: `no_min_unchanged`, and `claims.accounted`
  is never a default.
- New: mode keys `min_citations`, `status_tags`, `excuse`; the check
  `claims.accounted`; checklist `items_from = "candidate"` with
  `units = "blocks"`.

## Compatibility and reversibility

An older binary refuses a mode with the new keys (`value.form`): the
mode does not load there and its candidates are not ready. Removing the
keys returns the shipped behaviour; receipts of `claims.accounted` then
name a check the mode no longer lists and are ignored.

## Alternatives

- **`min_citations` alone.** One token then covers any amount of text.
- **A model that marks claims.** Writfold starts no model, and the
  result would be a judgment shown as a check.
- **Sentences as the unit.** Sentence splitting is a parser with
  opinions; blocks are what the Markdown reader already yields.

## Evidence and tests

- Kernel tests; proofs gate; claims refresh.
- `crates/writfold-kb/tests/deliver_claims.rs`: a draft with no tokens
  fails under `min_citations = 1` and passes without it; every class;
  a misspelt tag; a stale citation fails, an unverifiable one is
  unknown; an excused block, then the block edited (the row is stale
  and the block bare again); headings and code blocks are not counted;
  an empty draft is unknown.
- The foreign-member test: a block of every Markdown kind the reader
  yields is either classed or named as not a block in the note.
- Guides: [delivery-modes](../guide/delivery-modes.md),
  [research](../guide/research.md), changes.md, design.

## Rollout

One issue (#189): `min_citations` first (small, with `cited`), then
`claims.accounted` and the excuse checklist.

## Unresolved questions

1. **Owner:** the built-in tag words. *Default: none built in; a mode
   lists its own, and the example uses `inferred`, `recalled`,
   `unknown`.*
2. **Owner:** may a tagged block count towards `min_citations`?
   *Default: no; a tag is not a citation.*

## Decision log

- 2026-11-27: drafted from report 2's fourth ask. Accepted under the
  owner's standing acceptance; not yet reviewed by the owner.
- 2026-11-28 (as built, #189): `writfold_core::claims` with `accounted`,
  `cited` and two helpers (`fails`, `unsure`), because Aeneas does not
  expand the nested enum match the draft wrote. The mutants `bare-passes`
  and `unverifiable-passes` therefore break `fails_spec` and `unsure_spec`;
  `empty-passes` breaks `accounted_spec`. `accounted` is linked to
  `Calculus.Universal` with the empty universe allowed (an all-excused
  draft passes; the kernel keeps an empty one unknown), `cited` to
  `Calculus.Tri.and`. `min_citations` and `claims.accounted` with citations
  and status tags are built; the reviewer's excuse checklist is split to
  #221. A grammar finding (a near-miss tag, two tags, a tag on a cited
  block) leaves the block bare, so the fail is the kernel's. Blocks are read
  by a new `markdown::blocks`; `paragraphs` is unchanged.
