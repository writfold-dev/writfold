---
rfc: "2026-11-27-asset-boundary"
title: "Assets and how they are validated: identity, a declared line, and facts from a provider; Git is the first provider, not the model"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/195
milestone: "v0.6.0-dev.26"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Assets and how they are validated

Foundation for [declared repositories](2026-11-27-declared-repositories.md)
(accepted) and [providers report facts](2026-11-27-providers-report-facts.md)
(draft). It places [citation target](2026-11-27-citation-target.md) (#179)
and [partial clones](2026-11-27-partial-clone-identity-first.md) (#182)
under one boundary and applies law R of
[referents, remainders and sites](2026-11-27-referents-remainders-sites.md).

## Summary

The owner asked whether fixing the false `gone` (#179) ties evidence
validation to Git remotes, refs and clone layouts, and whether there is a
better model for assets as a whole.

The answer has two halves.

**Git is the right dependency in three places and stays there:** the KB's
own storage, a run's subject (the tree a gate ran on), and the first
provider of cited code. None of this RFC moves them.

**Git is in two places by accident, and those are what #179 exposed:**

1. Which line of a cited repository counts as "now" is picked by a naming
   convention (`origin`, `HEAD`), and after #179 by inference from each
   token's text. It should be data: a declaration in Git text, the local
   map, and only then inference that says it is inference.
2. There is nothing between "a Git clone, judged by a proved kernel" and
   "an external checker that answers a verdict word". An asset outside Git
   gets no proved judgment, no content identity that Writfold compares,
   and no way to say "I cannot tell which thing is meant".

The verdict kernel is already independent of Git: `verdict::observed`
takes facts ("the same bytes at the cited name", "other bytes", "nothing
there, and how following it ended") and reads no repository. What is
missing is a named boundary around it. This RFC names it:

> **Law A.** A verdict on a cited asset is
> `judge(resolved, relation, fact)`. A **provider** supplies those three
> inputs for one sort of repository and never a verdict. What a provider
> must choose (which replica, which line) it chooses from data, in a
> stated order; a choice the data does not settle is `ambiguous`; the rule
> that chose is printed with the verdict.

Nothing shipped is withdrawn and no token changes. #179 resumes now, with
its kernel split in two (below); it is the Git provider's resolver.

## Proof obligations

- **Referent and resolver:** the referent is the snapshot a citation is
  judged against; each provider has one resolver module (`target::choose`
  for Git). **Universe:** none claimed. **Site:** none.
- **Kernel:** `writfold_core::referent`, provider-neutral.

  ```rust
  /// How the cited snapshot relates to the one judged against.
  pub enum Relation { Contains, Behind, Diverged, Unknown }
  /// What a referent can support (moved unchanged from #179's `target`).
  pub fn settle(resolved: bool, relation: Relation, verdict: Verdict) -> Verdict;
  /// The one path from a provider's facts to a verdict.
  pub fn judge(resolved: bool, relation: Relation, fact: Observed) -> Verdict;
  ```

  `judge(r, rel, f) = settle(r, rel, verdict::observed(f))`. `settle` is
  #179's table: unresolved gives `Unverifiable`; `Behind` turns `Changed`,
  `Gone`, `Moved` and `Renamed` into `Unverifiable`; otherwise the input.
- **Theorems** (`proofs/Writfold/Referent.lean`):
  - `settle_spec`, `ambiguous_never_judged`, `behind_never_stale`,
    `fresh_survives_behind`, `settle_identity`: the five #179 lists for
    `settle`, moved here.
  - `settle_only_withholds`: for every input `settle` returns its input or
    `Unverifiable`. The boundary takes verdicts away; it never makes one.
  - `judge_spec`: `judge` is the composition, for every input.
  - `judge_fresh_needs`: `judge = Fresh` implies `resolved` and
    `observed(fact) = Fresh`.
  - `judge_refines`: replacing an unresolved or `Behind` referent by a
    resolved one that is not behind, with the same fact, changes the
    verdict only where it was `Unverifiable`. With `history_only_refines`
    (#90, shipped) and `fetch_only_refines` (#182), this is the general
    form: a provider that knows less never contradicts one that knows
    more.
- **Mutants:** `behind-keeps-gone`, `ambiguous-judged`, `behind-drops-fresh`
  (from #179, against `settle_spec`); `judge-skips-settle` (against
  `judge_spec`).
- **Tested and unproved:** everything a provider does. For Git: finding
  the replica, reading remotes, URL normalisation, tips, ancestry, blob
  ids and bytes (#179, #182, the existing `stale` tests). One test greps
  the product crates: `verdict::observed` is called for a code or gate
  citation only inside `referent`.

No protocol changes, so no TLA+ model. Tracked in #196.

## What an asset is

An **asset** is anything a citation or a pin points at as evidence: a
span of a source file, a gate receipt, a file in an attached store, a
ticket, a page, a data file, a build output. (The KB's *inventory assets*
under `inventory/assets/` are records about things in the world. They are
one sort of thing cited, through `kind=record`, and are not what this RFC
means by the word.)

Citing an asset involves five things. Only the first three belong in the
citation.

| | Question | In a code token today | Where it lives |
|---|---|---|---|
| **Identity** | which bytes | `blob=` (the Git name of the file's bytes), `span=` (SHA-256 of the cited lines, trimmed) | the citation |
| **Name** | where in what | `repo=`, `path=`, `L=` | the citation |
| **Snapshot** | as of which state | `rev=` | the citation |
| **Replica** | where a copy is on this machine | the local map, `$WRITFOLD_KB_REPOS`, a path | this machine, untracked |
| **Line** | which moving reference counts as now | `origin/HEAD`, else `HEAD` | nowhere: a convention in code |

Identity is intrinsic: anyone holding the bytes can recompute it. Name
and snapshot are context. Replica and line are not properties of the
citation at all. The replica is a fact of one machine. The line is a fact
of the cited repository that every machine should agree on, and it is the
only one of the five with no home in data.

### Which parts are Git's

| Layer | Git by necessity? | Notes |
|---|---|---|
| Identity | no | A Git blob name is a pure function of bytes: SHA-1 or SHA-256 over `blob <length>\0` and the bytes. Software Heritage's `cnt` identifier and the `gitoid` URI scheme use the same construction with no repository involved [1][12]. Two accidents: the name depends on the repository's object format (the same file has a 40-hex and a 64-hex name; `writfold-runtime` reads both; by `other_blob`'s table a token from one format compared in the other falls to the span search, never to a false `fresh`; read, not run) [11], and nothing outside Git reports it. Registries, HTTP servers and buckets report SHA-256 [4][10] |
| Name | no | a named collection, a path, a range |
| Snapshot | the value, not the idea | every version-control system has an immutable revision id; a directory or a URL has none |
| Replica | only that it is a clone | |
| Line | **no: pure convention** | the mission needs a "now" ("marked when that source has moved"). In a distributed system no clone can know which copy is authoritative; a team can, and says it once |
| History (renames, ancestry, reachability) | Git-shaped, and optional | history only refines a verdict (`history_only_refines`); a provider without history decides less, never differently |
| The KB's storage | **yes**, by the mission | one record per file merges in Git; the index is rebuilt at a commit; the accepting identity is the commit's |
| A run's subject | **yes** | a candidate is a Git tree; `gate verify` re-runs at a commit; `handoff check` reads policy at a revision |

### What is shipped, on those layers

Read at `d2ae5690` (the guides and `verdict.rs`, `store.rs`, `stale/`).

| Asset | Identity | Line | Who decides |
|---|---|---|---|
| `kind=code`, any repository | `blob=`, `span=` | `origin/HEAD`, else `HEAD` | the kernel, from Git facts |
| `kind=code` into a pinned store | `blob=` | none: its own `rev=`, while a ref reaches it | the kernel (`store`) |
| `kind=gate` | the receipt's commit | the same convention | the kernel |
| `kind=record`, `proposal` | `state=` | the KB's index | the kernel |
| kinds by time (`web`, `ticket`, …) | none | none | the kernel, from `seen=` and a lifetime |
| `immutable` kinds | none | none | always fresh |
| `checked` kinds | the checker's own fields | the checker's | **the checker**: it answers the verdict word |
| a snapshot of a look (`observe --snapshot`) | SHA-256 in the provenance file, then a code citation | the pin | the kernel |
| a gate receipt's subject | the scope's content digest and the definition digest | the work tree now | the gate kernels |

Two observations. Content identity already decides most of this (blob
names, scope digests, snapshot digests), so the model is closer to
content addressing than its vocabulary suggests. And the pinned store
already shows that "now" is optional: where the question is "do the
cited bytes still exist", no line is needed and no fork can mislead.

## Prior art

Every system below separates the same things. None was adopted whole;
each row says what is taken.

| System | Identity | Location | Moving reference | Taken |
|---|---|---|---|---|
| SWHID (ISO/IEC 18670:2025) [1] | core identifier, "intrinsic": computed from the object alone | `origin` qualifier | `visit`, `anchor` qualifiers; resolution is out of scope | the split of a core identifier from context qualifiers. A code token is nearly a qualified SWHID: `blob` is `cnt`, `rev` the anchor, `url` the origin, `path`, `L` the `lines` fragment. An export is cheap and is not planned |
| in-toto attestation v1, SLSA v1.0 [2][3] | `digest`, a set keyed by algorithm (`sha256`, `gitBlob`, `gitCommit`, …) | `uri`, `downloadLocation` | SLSA records the ref in the `uri` and the commit it resolved to in `digest` | the digest set keyed by algorithm; the pair "what was asked for, what it resolved to" |
| Nix flakes [6] | `locked`: `rev`, `narHash` | fetcher types (`git`, `github`, `tarball`, `path`, `mercurial`, …) | `original`, re-resolved only by `nix flake update` | the closest match to the recommendation: fetchers are providers; the lock is identity; the moving reference is declared once, beside the lock, and resolving it is an explicit act |
| OCI image and distribution specs [4] | `digest` as `algorithm:encoded` | `urls` | a tag points at a manifest; a digest names it | the textual digest form `sha256:<hex>` |
| Bazel `http_archive`, remote execution CAS [7] | `sha256` or `integrity`; `Digest{hash, size_bytes}` | `urls`, mirrors tried in order | none: a URL without a digest is called a risk | several locations for one identity (`mirrors`) |
| HTTP [10] | `Repr-Digest` (RFC 9530); a strong `ETag` (RFC 9110); `ni:` names (RFC 6920); Subresource Integrity | the URL | the URL now; Memento (RFC 7089) for a state at a time | what a provider for URLs can observe without downloading: a digest field or a strong validator. A weak validator decides nothing |
| purl (ECMA-427) [5] | none (`checksum` is a recommended qualifier) | `repository_url`, `vcs_url`, `download_url` | a version | not an identity of bytes. Useful later as a spelling for `name` |
| IPFS CID, multihash [8] | a digest that carries its algorithm | none | none | the principle that a digest names its function. The binary encodings are not taken: a token is read and grepped by people |
| W3C PROV-DM [9] | an Entity | none | `specializationOf`: a dated state is a specialization of the thing that evolves | the vocabulary for law R: a citation pins a specialization, a stale verdict is about the general entity, and a fork is an `alternateOf` it. No RDF |
| Git LFS, DVC [13] | `oid sha256:…` and a size in a small text pointer | a remote | none | large data and model outputs are cited by a pointer that holds a digest; no bulk bytes in the KB |
| Mercurial, Perforce, Jujutsu [14] | changeset hash; `p4 fstat` digest; commit id | the server or clone | a bookmark; a depot head; a branch | "a path at a revision with a content digest" is common to all of them; remotes and a default branch are Git's |

## The boundary

**Three inputs, one judge.** For a citation with a content identity a
provider answers three questions about the present and hands the answers
to `referent::judge`:

| Input | Outcomes | The Git provider |
|---|---|---|
| **Resolution**: which snapshot is "now" | resolved, with the rule that chose; ambiguous, with why; none | `target::choose` (#179), fed by the declaration (#197), the local map and the clone's remotes |
| **Relation** of the cited snapshot to it | `Contains`, `Behind`, `Diverged`, `Unknown` | one ancestry check per cited commit (#179). A provider without history answers `Unknown` |
| **Fact** at that snapshot | `verdict::Observed`: the same bytes at the cited name; other bytes, and where the span sits; nothing there, and how following it ended | blob ids from trees, bytes only where the ids differ (#182) |

A pinned citation is the degenerate case: its referent is its own
snapshot, so it is resolved and `Contains` by construction.

**Providers, in three tiers.** The tier says how much of the decision is
proved, and each verdict already prints who checked it.

1. *In the binary, local bytes only.* Git, shipped. A plain directory,
   planned (#202). The binary stays static and never opens a socket.
2. *An external program that reports facts.* Anything behind a network
   or a credential: another version-control system, a URL with a digest
   field, a registry, a bucket. Planned as an `observe` operation beside
   `check` (#201). The program is found on `PATH` as checkers are, so the
   KB holds no command, URL or secret, and no server joins the product
   path.
3. *An external program that answers a verdict word* (`check`), and
   *time* (`ttl`). Both shipped, both stay. They suit assets with no
   bytes to compare (a ticket's state, a dashboard). They are the lowest
   tier: the decision is the program's.

**Choices come from data, in order.** For the Git provider:

1. the local map's `ref=` (#179): this machine's owner said so;
2. the repository's declaration in the KB (#197): the team said so;
3. inference from the tokens' `repo=` and `url=` and the clone's remotes
   (#179's ladder), printed as the rule that chose;
4. otherwise ambiguous, with the line to add.

**What a citation needs to say, and no more:** identity, name, snapshot.
A token never names a remote, a branch or a clone path. `url=` stays a
hint for finding a replica. Tokens that predate this RFC are complete.

**Verdicts travel as check receipts.** A machine without the provider
reads a recorded verdict under the shipped rules (`max_age`,
`recorded_max_age`, a local verdict always wins). That part of the asset
story needs no change.

## How the open work fits

| Work | Under the boundary |
|---|---|
| #179 citation target | the Git provider's resolver. `settle` and `Relation` move to `referent` (#196); `choose` stays in `target`. Behaviour as specified |
| #180 `cite make`, `decision --commit` | the provider's `make`, at the same resolved referent; extended by #198 once a repository can be declared |
| #182 partial clones | the Git provider's fact gathering: identity first, bytes only when ids differ. Unchanged; `fetch_only_refines` is an instance of `judge_refines`'s family |
| #190 convention-literal check | names `writfold_core::target` and `stale/repos.rs` as the only modules that may say `origin`, `refs/remotes` or `HEAD` for citations |
| #4 recorded pinned verdicts, check receipts | how verdicts travel; unchanged |
| attached stores | a declared repository with `freshness = "pinned"`; its clone is matched by any remote at the declared URL, not only `origin` (#198) |

## The paused kernel

Keep it, change its packaging, resume it now.

- **Keep:** `choose` with its types and every theorem and mutant #179
  lists, and `settle` with its table. The reproduction and the acceptance
  list of #179 stand. A fork with its own commits has diverged from the
  cited commit, so `settle` alone would not fix it: `choose` is needed.
- **Change:** `settle`, `Relation` and `claims_change` move to
  `writfold_core::referent` with `judge` (#196). They name nothing of
  Git, and the next provider reuses them and their proofs unchanged.
  `--json` gains `target_rule` in place of #179's `target_remote`: the
  rule that chose (`map-ref`, `declared`, `remote-url`, `remote-path`,
  `remote-base`, `sole-remote`, `head`). A remote's name is a detail of
  one provider and appears in the text of `target_ref`.
- **Known limit, closed by #197:** by `choose`'s table a token minted by
  today's `cite make` in a fork clone carries the fork's URL in `url=`,
  so that fork's remote classifies `Url` and outranks the upstream's
  `Path`. The token is judged against the repository its text names. A
  declaration overrides what tokens happen to say. (Read from the RFC's
  table; the kernel is not wired, so this was not run.)
- **Drop:** nothing.

## Behaviour

This RFC changes no output by itself. #196 lands with #179 and adds
nothing a user sees beyond #179's lines. What #197 and the draft add is
in their RFCs.

## Compatibility and reversibility

No file format, token or schema changes here. `referent` is a move of
unmerged code. Reverting is deleting a module that only #179's wiring
calls. Every later slice is additive: a KB with no declaration and no
observed kind reads what #179 reads.

## Trade-offs, stated plainly

- **A declaration is one more file to keep right.** A wrong URL in it
  makes every clone read `target-ambiguous` until fixed. That is the
  intended failure: loud, never a false `gone`. A KB that declares
  nothing keeps working by inference.
- **Inference stays.** Removing it would break every undeclared KB with
  more than one remote. It is kept, labelled, and ranked last.
- **A non-Git provider decides less.** Without history there is no
  rename following and no `target-behind`; more citations read `changed`
  or `gone` where Git would read `moved`. The verdicts it does give are
  the same ones.
- **External fact providers are user code.** Writfold proves what it
  does with their facts, not that the facts are true. The same holds for
  `git` today.
- **No abstraction is built before its second user.** The Git reader is
  not rewritten behind a trait. The boundary is one kernel function and
  a rule about who may call `verdict::observed`. A trait appears when
  the directory provider needs it.
- **Not everything has bytes.** A ticket's state or a dashboard has no
  digest worth comparing; time and `check` remain the honest tools.

## Alternatives

- **Do nothing beyond #179.** It fixes the report. It leaves the line
  undeclared, makes token text the authority on which repository is
  meant, and leaves every non-Git asset in the lowest tier.
- **Pin everything; drop "now".** Content addressing alone answers "do
  the cited bytes exist" and removes remotes and refs from validation
  altogether. It cannot answer "has the project moved since", which is
  the mission's sentence. It stays as one basis (`freshness = "pinned"`),
  chosen per repository.
- **A version-control abstraction in the binary** (Git, Mercurial,
  Perforce, Jujutsu behind one trait). Each is a native dependency and a
  test matrix, and their history models differ. The external fact
  protocol reaches all of them without one new crate. A Jujutsu
  repository with a Git backend is already a Git clone.
- **Resolve through an archive** (Software Heritage) or a transparency
  log. It needs the network on a read and covers public code only.
- **Signed attestations as the evidence model** (in-toto, SLSA). The
  mission defers signing until a multi-party need appears (#170). The
  vocabulary is taken; the envelope is not.
- **Move the KB itself off Git.** Tried with Dolt and Omnigraph and
  archived on 16 November 2026: both only projected the Git records.
- **Built-in HTTP or registry providers.** They would open sockets from
  a read verb and add crates the dependency checks reject. External
  programs do it with the operator's own credentials.

## Evidence and tests

- #196: the kernel, the proofs gate, `cargo tauten proofs`; the grep test
  for `verdict::observed`; #179's fork tests pass through `judge`.
- The existing fixtures read the verdicts of v0.6.0-dev.20 for a clone
  with one remote.
- The foreign-member discipline of #190 applies to each resolver: a
  remote the resolver does not recognise must be counted or change the
  answer.

## Rollout

Migration first; new capability only after an owner answer.

| Order | Issue | What | Sort | Needs |
|---|---|---|---|---|
| 1 | #196 | `referent::settle` and `judge` | migration | — |
| 1 | #179 | the Git resolver wired through `judge`; the map's `ref=`. **This unblocks the reporting team** | fix | #196, same branch |
| 2 | #180 | `cite make` and `decision --commit` at the same target | fix | #179 |
| 3 | #197 | declared repositories | new, small | #179 |
| 4 | #198 | `cite make`, stores and repository names read the declaration | migration | #197, #180 |
| with 1 to 4 | #199 | the asset model in the guides; a mission line | docs | — |
| later | #200, #201, #202 | tagged digests, `observe`, a directory provider | new capability | owner question 1 (#203); the draft RFC |

## Unresolved questions

None blocks slices 1 to 4. Owner questions, with the default taken, are
collected in #203. Those that touch this RFC:

1. **Owner:** which situation outside Git comes first, and when?
   *Default: none is built until a real case is named; the draft RFC
   stays a draft.*
2. **Owner:** providers that need a network stay outside the binary, as
   programs on `PATH`. *Default: yes.*
3. **Owner:** a run's subject stays Git only. *Default: yes; no other
   subject is planned.*
4. **Owner:** the mission gains one dated line: Git is required for the
   KB and a run's subject; a cited asset is validated through a
   provider, of which Git is the first. *Default: yes, with #199.*

## Decision log

- 2026-11-27: drafted in answer to the owner's question on #179 and Git
  coupling, after reading the validation code and guides at `d2ae5690`
  and checking the prior art against primary sources the same day.
  Accepted under the owner's standing acceptance (21 November 2026); not
  yet reviewed by the owner. It moves half of #179's unmerged kernel to
  `writfold_core::referent` and replaces #179's JSON key `target_remote`
  by `target_rule`; both are recorded in that RFC's decision log.

## References

Fetched on 27 November 2026.

1. SWHID specification v1.2, <https://www.swhid.org/specification/v1.2/>
   (published as ISO/IEC 18670:2025). Clause 3.4 defines an intrinsic
   identifier; clause 6 the qualifiers. It calls Git compatibility
   "practical, but incidental".
2. in-toto attestation v1: ResourceDescriptor and DigestSet,
   <https://github.com/in-toto/attestation/blob/main/spec/v1/resource_descriptor.md>,
   <https://github.com/in-toto/attestation/blob/main/spec/v1/digest_set.md>.
3. SLSA provenance v1.0, <https://slsa.dev/spec/v1.0/provenance>.
4. OCI image descriptor,
   <https://github.com/opencontainers/image-spec/blob/main/descriptor.md>;
   distribution specification,
   <https://github.com/opencontainers/distribution-spec/blob/main/spec.md>.
5. Package-URL, ECMA-427,
   <https://ecma-international.org/publications-and-standards/standards/ecma-427/>.
6. Nix flakes, lock file format,
   <https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-flake.html>.
7. Bazel `http_archive`, <https://bazel.build/rules/lib/repo/http>; the
   remote execution API's `Digest`,
   <https://github.com/bazelbuild/remote-apis>.
8. CID, <https://github.com/multiformats/cid>; multihash,
   <https://github.com/multiformats/multihash>.
9. W3C PROV-DM, <https://www.w3.org/TR/prov-dm/>, sections 5.5.1
   (specialization) and 5.5.2 (alternate).
10. RFC 9110 section 8.8 (validators), RFC 9530 (digest fields), RFC 6920
    (`ni:`), RFC 7089 (Memento), at <https://www.rfc-editor.org/>;
    Subresource Integrity,
    <https://www.w3.org/TR/2016/REC-SRI-20160623/>.
11. Git: the hash function transition,
    <https://git-scm.com/docs/hash-function-transition>; planned breaking
    changes, <https://git-scm.com/docs/BreakingChanges>; the 2.48 release
    notes on a missing `refs/remotes/<remote>/HEAD`,
    <https://www.kernel.org/pub/software/scm/git/docs/RelNotes/2.48.0.adoc>.
12. The `gitoid` URI scheme,
    <https://www.iana.org/assignments/uri-schemes/prov/gitoid>; OmniBOR,
    <https://github.com/omnibor/spec>.
13. Git LFS pointer files,
    <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>; DVC
    files, <https://doc.dvc.org/user-guide/project-structure/dvc-files>.
14. Mercurial changeset ids,
    <https://wiki.mercurial-scm.org/ChangeSetID>; `p4 fstat`,
    <https://help.perforce.com/helix-core/server-apps/cmdref/current/Content/CmdRef/p4_fstat.html>;
    Jujutsu glossary, <https://docs.jj-vcs.dev/latest/glossary/>.
