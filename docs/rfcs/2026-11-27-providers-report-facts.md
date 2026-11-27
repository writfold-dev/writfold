---
rfc: "2026-11-27-providers-report-facts"
title: "Providers report facts and the kernel judges: digests that name their function, an `observe` operation for checkers, a directory provider"
status: draft
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/201
milestone: null
supersedes: []
superseded_by: []
blocked_on:
  - "owner question 1 of https://github.com/writfold-dev/writfold/issues/203: which situation outside Git comes first"
---

# RFC: Providers report facts and the kernel judges

The new capability under [the asset boundary](2026-11-27-asset-boundary.md).
A draft: nothing here is built on its authority. It is written now so
that the boundary is shown to hold for a provider that is not Git, and so
that the first real case outside Git starts from a design, not from a
convention.

## Proof obligations

- **Referent and resolver:** the snapshot a provider reports as "now";
  the resolver is the provider. **Universe:** none claimed. **Site:**
  none.
- **Kernel:** `writfold_core::digest`.

  ```rust
  /// One algorithm both sides carry: are the two values the same bytes?
  pub struct Pair { pub equal: bool }
  pub enum Compared { Same, Other, Incomparable, Conflict }
  /// Over every algorithm the citation and the observation share.
  pub fn compare(shared: &[Pair]) -> Compared;
  ```

  No shared algorithm is `Incomparable`. All equal is `Same`. All unequal
  is `Other`. Some equal and some not is `Conflict`: one side is wrong
  about what it hashed.

  `fact(compared, located) -> Observed`: `Same` is
  `Code(Target::SameBlob)`; `Other` is `Code(Target::OtherBlob(true,
  located))`; `Incomparable` and `Conflict` are `Unresolved`. The verdict
  is then `referent::judge` of #196, unchanged.
- **Theorems** (`proofs/Writfold/Digest.lean`):
  - `compare_spec`: the table, for every list.
  - `same_needs_common`: an empty list never reads `Same` or `Other`. Two
    digests made by different functions decide nothing.
  - `conflict_never_same`: one unequal pair is enough to stop `Same`.
  - `fact_spec`; `incomparable_never_decides` (through `judge`, never
    `Fresh`, `Changed`, `Gone`, `Moved` or `Renamed`).
  - `observe_never_promotes`: for an observed kind, `Fresh` implies
    `compare = Same` at a resolved referent.
- **Mutants:** `any-match-wins` (a conflict reads `Same`), `empty-is-same`,
  `empty-is-other` (each breaks `compare_spec`); `conflict-is-other`
  (breaks `fact_spec`).
- **Tested and unproved:** the `digest=` field parser and hex case; the
  `observe` reply reader, where every malformed reply is `checker-failed`
  as for `check`; the directory walk, its limits and its hashing; the
  strings.

No protocol with crash points: one request and one reply per token, as
`check` has today. No TLA+ model.

## Problem

Today an asset outside Git has two options. By time: no identity at all.
Or a `checked` kind: the checker answers `fresh`, `moved`, `changed` or
`gone`. Each checker then holds its own unproved copy of the decision
the kernel makes for Git, compares digests in a field Writfold cannot
read, and has no way to say "I could not tell which thing is meant"
other than `unverifiable` with prose. Law R and law A stop at the
checker's door.

`blob=` cannot carry the identity either: it is a Git object name. A
registry, an HTTP server with `Repr-Digest`, a bucket or `p4 fstat`
report other functions (references 4, 10 and 14 of the foundation RFC).

## Decision

Three additions, each usable alone.

**1. A digest that names its function** (#200). A kind declared
`freshness = "observed"` carries `digest=<alg>:<hex>`, lower-case, one or
more. `sha256` is the function Writfold computes itself. `gitblob-sha1`
and `gitblob-sha256` name what `blob=` holds, so a provider that can
compute a Git blob name (any program with the bytes) compares with
existing code tokens. The form is OCI's `algorithm:encoded`; the set keyed
by algorithm is in-toto's. Built-in kinds are untouched: `blob=` and
`span=` keep their meaning and no token is rewritten.

**2. `observe`, beside `check`** (#201). `writfold-checker/1` already
negotiates operations through `describe`'s `ops`, so this is additive:

```text
→ {"v":1,"op":"observe","id":"0","kind":"art","fields":{"name":"reports/q3.parquet","digest":"sha256:9f2c…","rev":"v41"}}
← {"v":1,"id":"0",
   "referent":{"state":"resolved","id":"v44","rule":"bucket versioning: latest"},
   "relation":"contains",
   "object":{"state":"present","digest":{"sha256":"9f2c…"}}}
```

- `referent.state` is `resolved`, `ambiguous` or `none`; the last two
  carry `why`, shown as `target-ambiguous: <why>`.
- `relation` is `contains`, `behind`, `diverged` or `unknown`. A provider
  without history says `unknown`.
- `object.state` is `present` (with digests) or `absent`. With `absent`
  the provider may add `followed`: `renamed` with a path, `no-rename`, or
  nothing, which reads as "not followed" and so never as `gone`.
- A provider that searched for the cited span may add `located`.

Writfold compares the digests, builds the fact and calls `judge`. A
checker that lists only `check` works as before. Who runs a checker,
`max_age`, `--cached`, `--offline` and check receipts do not change.

**3. A directory provider in the binary** (#202). `provider = "dir"` on a
[declared repository](2026-11-27-declared-repositories.md). `kind=code`
tokens with `blob=` and `span=`, no `rev=`. The referent is the directory
as it is now: resolved, or none. The relation is `Unknown`. The same
bytes at the path are `fresh`. Other bytes follow the span search. A
missing path whose blob is found elsewhere in the tree is `moved`; found
nowhere, after a completed walk, `gone`; a walk that hit its limit is
`unverifiable`. No Git process is started, and nothing is fetched.

**Not included:** a provider built into the binary that opens a socket;
fetching; a provider for the built-in `record` and `gate` kinds; moving
the Git reader onto the protocol; token migration.

## Compatibility and reversibility

An older binary reads a kind declared `observed` as an unknown freshness
and refuses the declaration, as it refuses any bad value today, so a KB
adopts an observed kind only once its binaries understand it. It reads a
repository declared `dir` as it reads any declaration it does not know:
not at all (the directory is new with #197). Tokens of an observed kind
are ordinary tokens of a custom kind.

## Alternatives

- **Keep verdict words only.** Every checker stays a private copy of the
  decision, and `target-behind` and `target-ambiguous` are unreachable
  outside Git.
- **`writfold-checker/2`.** Not needed: `ops` already negotiates.
- **Multihash or `ni:` for digests.** Self-describing, and unreadable in
  a token that people grep. The textual `alg:hex` carries the same
  information.
- **Send the bytes to Writfold and let it hash.** Honest, and it makes
  every check a download. A provider may still do so by reporting
  `sha256` it computed; the trust is the same as trusting `git`.

## Evidence and tests

The kernel with exhaustive tests and the proofs gate. A template checker
in the checkers guide that serves a directory over `observe`, run by a
test as the `check` template is. The existing span and rename fixtures
exported to plain directories and compared with their Git clones.

## Rollout

Not scheduled. In order when it is: #200, then #201 or #202, whichever
the first real case needs.

## Unresolved questions

1. **Owner:** which situation outside Git comes first: another
   version-control system, files in a bucket or registry, a plain
   directory, or none yet? *Default: none; this stays a draft.*
2. **Owner:** the mission (13 November 2026) lists "built-in kinds
   through the protocol" and a checker SDK as not built. This RFC builds
   neither. Does `observe` count as checker phase 2, which waits on real
   use of phase 1? *Default: yes, it waits.*

## Decision log

- 2026-11-27: drafted with the asset boundary RFC. Left as a draft,
  blocked on owner question 1 of #203: it is new capability with a
  protocol surface, and no user has named the first case.
