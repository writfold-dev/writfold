---
rfc: "2026-11-22-missing-history"
title: "Missing history is not evidence of change"
status: accepted
implementation: complete
authors:
  - fable
  - opus
created: 2026-11-22
updated: 2026-11-22
issue: https://github.com/writfold-dev/writfold/issues/90
milestone: "v0.6.0-dev.17"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Missing history is not evidence of change

## Proof obligations

The decision moves into the kernel, `writfold_core::verdict`. `observed`
on a code citation now calls `other_blob(known, located)`, where `known`
says whether a cited blob was compared at all. It also calls
`missing(absence)`, where `absence` records how following an absent path
ended: an exact rename, followed (with where the span sits), no rename, the
cited commit not in this clone, no `rev=`, or over the per-run cap.
`proofs/Writfold/Verdict.lean` proves:

- `other_blob_spec`, `missing_spec`, `observed_spec`: each function is its
  table.
- `absence_never_evidence`: when the follow could not run (`RevAbsent`,
  `NoRev`, `Capped`), the citation reads unverifiable.
- `unknown_blob_changed_only_by_span`: with no cited blob known, a citation
  reads changed only when a span search ran and did not find the span, and
  never reads gone.
- `gone_witness`: gone holds exactly when the path is absent and the follow
  completed, either with no rename or with a rename whose span was not
  located.
- `changed_witness`: changed holds exactly when a known cited blob differs,
  or when a span search ran and did not find the span.
- `history_only_refines`: knowing the cited blob changes a verdict only
  where it was unverifiable. Every verdict decided from the bytes at the
  target is the same with or without history.
- `rename_never_fresh`, formerly a unit test in `rename.rs`.
- `observed_fresh_iff` and the mark theorems still hold. `with_mark` is
  unchanged, so an authored mark now shows on citations that used to read
  gone or changed for want of history.

Each new mutant breaks exactly its function's table theorem:
`no-blob-no-span-changed`, `rev-absent-gone`, `capped-gone`,
`followed-span-fresh`. `moved-span-fresh` and `unsearched-fresh` now edit
`other_blob`, and `same-blob-changed` covers `observed` itself.

**Tested, not proved:**
- shallow detection (`<git dir>/shallow`, no Git spawn);
- `kb_renames` telling an unknown commit (Git's "bad object" or "unknown
  revision") from every other failure, which stays `git-error`;
- whether the cited commit is present (`rev^{commit}` in the existing
  batch check);
- the reason strings and the `kb repos` rendering.

## Problem

From the groomed #90: `kb stale` reported `changed` or `gone` for a code
citation when the only fact was that this clone lacks the cited commit.
That happens with a shallow or single-branch clone, an unfetched or
rewritten commit, or a map entry pointing at the wrong clone. Three
neighbours made the same error:
- every `kb_renames` failure was read as "rev not in repository";
- the 64-revision cap reported `gone` for work not done;
- a token without `rev=` reported `gone`.

## Decision

A `changed` or `gone` code verdict needs positive local evidence.
Otherwise the verdict is `unverifiable`, with one of these reasons:
- `history-not-local`: the cited commit is not in this clone. The reason
  names a shallow clone and says to run `git fetch --unshallow`.
- `nothing-to-compare`: there is no `blob=`, no usable `rev=` and no
  `span=`.
- `rename-unchecked`: the follow was over the per-run cap.

`kb repos` adds a `history: shallow` line and a `shallow=N` count, and the
row status stays `local`. Verdicts decided from the bytes at the target are
unchanged. Out of scope: refusing shallow clones, fetching on a read, pin
rules and recorded verdicts.

## Consumers

The Explore survey of every reader of a code verdict:

| Consumer | Before | After |
|---|---|---|
| `kb stale` exit code | 2 | 2 (unverifiable also exits 2) |
| `kb stale` trailer | `stale=` | `unverifiable=` |
| `kb check` exit code | unchanged | unchanged |
| `proposal accept` | refused: "changed …" | still refused (exit 3), message names the reason |
| delivery check `claims.cited-fresh` | Fail | Unknown, unless another finding fails |
| gate pins | `searched-moved` | `unreadable`; `holds` is false either way |
| grounding | word "changed"/"gone" | word "unverifiable"; still ungrounded |
| session start | "N gone, M changed" | "K unverifiable" |
| `brief`, `context`, `impact` counts | counted | still counted; status word changes |
| watch and TUI | `!`, bad tone | `?`, warning tone |
| authored mark | ignored (gone and changed win) | the mark shows; stays stale and refused |
| pinned-store stand-in (#4) | not used | still not used: head-basis citations fail `on_pin` |

`kb stale --cached` keeps showing a cached verdict until a full `kb stale`
runs.

## Acceptance

`crates/writfold-kb/tests/shallow.rs` checks the same citations against a
full repository and a `--depth 1` clone, in both directions:
- a `blob=` token at an unchanged path is fresh in both;
- a `span=` token at a shifted file is moved in both;
- with no `blob=` or `span=`, a renamed and a deleted path read changed,
  moved and gone in the full clone, and `history-not-local (shallow clone)`
  in the shallow one;
- `kb repos` marks the shallow clone;
- after `git fetch --unshallow`, every row matches the full clone.

`rename.rs` covers the cap and the missing `rev=`; `kb_renames` has its own
test for telling failures apart.

## Decision log

- 2026-11-22: drafted from the groomed #90; accepted under the owner's
  standing acceptance; implemented in v0.6.0-dev.17.
