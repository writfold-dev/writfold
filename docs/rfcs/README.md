# Writfold RFCs

An RFC is the durable record of a decision that is costly to reverse or that
touches more than one part of Writfold, together with its evidence. Work is
tracked in [GitHub issues](https://github.com/writfold-dev/writfold/issues)
and [milestones](https://github.com/writfold-dev/writfold/milestones). An RFC
is the design behind an issue labelled `needs-rfc`. It never replaces the
issue.

The standing authorities sit above every RFC: the
[mission](../writfold/mission.md), the
[v1 design](../writfold/kb-v1-design.md) and the rules in
[AGENTS.md](../../AGENTS.md). Those rules are:
- Git text is the source of truth.
- The index can always be rebuilt.
- No server on the product path.
- Formats and citation tokens are preserved.
- Lean proofs come first.
- The size budgets apply.

An RFC explains why a design is worth what it will cost to maintain. It cannot
waive any of those rules.

## When an RFC is required

Write an RFC before implementing:

- a new verb, flag, exit code, output line or JSON key, or a change to one
  that an existing setup can notice (anything that would go in
  [changes.md](../guide/changes.md));
- a new or changed file format under the KB, a store or a run (a schema name,
  a record kind or a citation token);
- new or changed decision logic: what counts as met, stale, ready, skipped,
  complete, refused or allowed. Its proof obligations come first (below);
- a new dependency, runtime component or protocol that limits future design;
- an exception to a rule in [AGENTS.md](../../AGENTS.md).

Bug fixes, refactors that keep behaviour, dependency upkeep, documentation
corrections and gate or release plumbing do not need one. Use an issue and a
commit that names it.

## File and heading format

- Filename: `YYYY-MM-DD-kebab-title.md`. The date is the day drafting started
  and equals `created`. No number is allocated.
- Heading: `# RFC: Title`. The RFC id is the filename without `.md`.
- `0000-template.md` is reserved and is not an RFC.
- A retitled RFC keeps its filename. Revise the one file and record material
  changes in its decision log. Do not make `v2` or `final` copies.

## Frontmatter

```yaml
---
rfc: "2026-11-20-short-descriptive-title"
title: "Short descriptive title"
status: draft
implementation: not-started
authors:
  - handle
created: 2026-11-20
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---
```

Field rules:
- `status` is `draft`, `accepted`, `rejected` or `superseded`.
- `implementation` is `not-started`, `in-progress`, `partial`, `complete`,
  `removed` or `n/a`.
- `issue` is the tracking issue URL.
- `milestone` is the milestone title the work is planned for, or `null`.
- `blocked_on` names concrete dependencies, such as other RFC ids, issue URLs
  or an owner question. An RFC waiting on research is a `draft` whose
  `blocked_on` is not empty.
- `status` and `implementation` are recorded only in the frontmatter. The body
  may add a dated note when it says more.

## Lifecycle

```text
draft ──owner decision──▶ accepted ──accepted replacement──▶ superseded
  └───owner decision──▶ rejected
```

- **Draft:** under design. A draft may be merged to `main` so that review has
  one home, but merging it does not accept it, and nothing may be built on
  its authority.
- **Accepted:** the owner approved the decision and its boundaries.
  `implementation` then advances as work lands, with each commit naming the
  issue.
- **Rejected:** keep the file with a short rationale.
- **Superseded:** a later accepted RFC owns the decision. Link both ways and
  keep the old text.

Agents may draft RFCs and propose changes to them. The owner accepts or
rejects one ([GOVERNANCE.md](../../GOVERNANCE.md)). Since 21 November 2026
the owner accepts RFCs by default: an agent sets `status: accepted` with a
decision-log line saying so, implements it, and the owner reviews and
amends or rejects afterwards. Work is not held waiting for acceptance.

## Proof obligations come first

An RFC that changes decision logic lists, before its design:

1. The kernel in `crates/writfold-core`, with its inputs and outputs.
2. The theorems to be proved in Lean (`proofs/`) and the mutants that must
   break them.
3. What stays tested and unproved: parsers, file and Git IO, rendering. Say
   how each part is tested.

The proofs gate (drift, `lake build`, axiom audit, mutants) must pass before
the kernel is wired into a verb ([aeneas.md](../writfold/aeneas.md)).

## Process

1. Open or find the issue. The owner labels it `needs-rfc`.
2. Copy [the template](0000-template.md) to `YYYY-MM-DD-kebab-title.md`, fill
   in every frontmatter field, and add a row to the registry below.
3. Review the problem, behaviour, proof obligations, compatibility and
   rollout. Record material outcomes in the decision log.
4. The owner sets `status: accepted` or `rejected`. The tracking issue then
   becomes `accepted`.
5. Implementation commits name the issue (`Refs #N`, or `Closes #N` on the
   last one). They update `implementation` and, where the spec deviated, the
   RFC's as-built notes.
6. A later decision that is incompatible gets a new RFC that supersedes the
   old one.

## Specs written before this process

Before 20 November 2026, designs were written as specs under `docs/writfold/`.
They moved here on that day with frontmatter added. Their bodies are
unchanged, including their own status lines, which are historical. The
frontmatter is current. Two documents stay where they are and remain the
standing authority: the [mission](../writfold/mission.md) and the
[v1 design](../writfold/kb-v1-design.md), whose numbered sections are the
as-built record of every release.

## Registry

Rows are in creation order.

| RFC | Decision | Status | Implementation | Issues |
|---|---|---|---|---|
| [2026-11-13](2026-11-13-watch-run-progress.md) | T2: run progress in `writfold watch` | accepted | complete | — |
| [2026-11-14](2026-11-14-watch-plan-map.md) | T3: the plan map and node inspector in `writfold watch` | accepted | complete | — |
| [2026-11-15](2026-11-15-delivery-modes-as-step-gates.md) | Delivery modes as a step's gates | accepted | complete | — |
| [2026-11-15](2026-11-15-v050-dev10.md) | v0.5.0-dev.10 | accepted | complete | [#36](https://github.com/writfold-dev/writfold/issues/36) |
| [2026-11-15](2026-11-15-v050-dev9.md) | v0.5.0-dev.9 | accepted | complete | [#32](https://github.com/writfold-dev/writfold/issues/32), [#35](https://github.com/writfold-dev/writfold/issues/35) |
| [2026-11-15](2026-11-15-watch-calm-defaults.md) | `writfold watch` T4: calm defaults and progressive disclosure | accepted | complete | — |
| [2026-11-15](2026-11-15-watch-knowledge-base.md) | `writfold watch` T5: the knowledge base in the same frame | accepted | complete | — |
| [2026-11-15](2026-11-15-watch-plain-language.md) | `writfold watch` T6: a plain-language surface | accepted | complete | — |
| [2026-11-16](2026-11-16-diagnostics.md) | Diagnostics for the files a user writes | accepted | complete | — |
| [2026-11-16](2026-11-16-proof-backfill.md) | Proof backfill: what is proved, what is not, and the order to close it | accepted | partial | [#14](https://github.com/writfold-dev/writfold/issues/14), [#16](https://github.com/writfold-dev/writfold/issues/16), [#17](https://github.com/writfold-dev/writfold/issues/17) |
| [2026-11-16](2026-11-16-semantic-search-spike.md) | Semantic search spike | draft | not-started | [#27](https://github.com/writfold-dev/writfold/issues/27) |
| [2026-11-16](2026-11-16-user-defined-policy.md) | User-defined policy: accepted outcomes, unskippable steps, pins and the first-use path | accepted | partial | [#10](https://github.com/writfold-dev/writfold/issues/10) |
| [2026-11-17](2026-11-17-delivery-modes-second-round.md) | Delivery modes, second round | accepted | partial | [#12](https://github.com/writfold-dev/writfold/issues/12), [#13](https://github.com/writfold-dev/writfold/issues/13) |
| [2026-11-17](2026-11-17-find-next.md) | `find`: what to build next (measured) | accepted | partial | [#18](https://github.com/writfold-dev/writfold/issues/18), [#19](https://github.com/writfold-dev/writfold/issues/19), [#20](https://github.com/writfold-dev/writfold/issues/20), [#21](https://github.com/writfold-dev/writfold/issues/21), [#22](https://github.com/writfold-dev/writfold/issues/22), [#23](https://github.com/writfold-dev/writfold/issues/23), [#24](https://github.com/writfold-dev/writfold/issues/24), [#25](https://github.com/writfold-dev/writfold/issues/25), [#26](https://github.com/writfold-dev/writfold/issues/26) |
| [2026-11-17](2026-11-17-v060-dev6-and-attached-stores.md) | v0.6.0-dev.6 and attached stores | accepted | partial | [#38](https://github.com/writfold-dev/writfold/issues/38) |
| [2026-11-18](2026-11-18-attached-stores-next.md) | Attached stores, next: backlinks, snapshots and the deferred items | accepted | partial | [#2](https://github.com/writfold-dev/writfold/issues/2), [#3](https://github.com/writfold-dev/writfold/issues/3), [#4](https://github.com/writfold-dev/writfold/issues/4), [#5](https://github.com/writfold-dev/writfold/issues/5), [#6](https://github.com/writfold-dev/writfold/issues/6), [#39](https://github.com/writfold-dev/writfold/issues/39), [#11](https://github.com/writfold-dev/writfold/issues/11) |
| [2026-11-18](2026-11-18-gate-evidence-binding-and-pins.md) | Gate evidence: binding and pins | accepted | partial | [#8](https://github.com/writfold-dev/writfold/issues/8), [#9](https://github.com/writfold-dev/writfold/issues/9) |
| [2026-11-20](2026-11-20-citations-as-links.md) | Citations rendered as links in posted text | draft | not-started | [#5](https://github.com/writfold-dev/writfold/issues/5) |
| [2026-11-20](2026-11-20-gates-owed-by-diff.md) | Gates owed by what a change touches (applies_when) | draft | not-started | [#9](https://github.com/writfold-dev/writfold/issues/9) |
| [2026-11-20](2026-11-20-links-from-trailers-and-at-commit.md) | Links from commit trailers and tracker references, and answers as of a commit (M4b) | draft | not-started | [#31](https://github.com/writfold-dev/writfold/issues/31) |
| [2026-11-20](2026-11-20-recorded-pinned-verdicts.md) | Recorded verdicts for pinned store citations | accepted | complete | [#4](https://github.com/writfold-dev/writfold/issues/4) |
| [2026-11-20](2026-11-20-statement-order-without-clock.md) | Order statements without a clock | accepted | complete | [#15](https://github.com/writfold-dev/writfold/issues/15) |
| [2026-11-21](2026-11-21-reader-verdict-parity.md) | Show the usable checked-citation verdict on a cold read | accepted | complete | [#35](https://github.com/writfold-dev/writfold/issues/35) |
| [2026-11-21](2026-11-21-reviewed-lookup-misses.md) | Turn a reported lookup miss into a reviewable alias | accepted | not-started | [#22](https://github.com/writfold-dev/writfold/issues/22) |
| [2026-11-21](2026-11-21-complete-gate-inventory.md) | A gate inventory that could not be listed is not a complete answer | accepted | complete | [#56](https://github.com/writfold-dev/writfold/issues/56) |
| [2026-11-21](2026-11-21-strict-handoff-check.md) | Judge a handoff against an operator's required gates, not the agent's plan | accepted | complete | [#57](https://github.com/writfold-dev/writfold/issues/57) |
| [2026-11-21](2026-11-21-streamed-verify-same-pin-repair.md) | Stream index rebuild and verification; repair only the generation compared, at a still-current pin | accepted | complete | [#58](https://github.com/writfold-dev/writfold/issues/58) |
| [2026-11-21](2026-11-21-refresh-affected-graph-facts.md) | Refresh only the graph facts an edit affects | accepted | complete | [#59](https://github.com/writfold-dev/writfold/issues/59) |
| [2026-11-21](2026-11-21-keyed-graph-refresh.md) | Refresh the graph by token when a record is added, removed or renamed | accepted | complete | [#65](https://github.com/writfold-dev/writfold/issues/65) |
| [2026-11-21](2026-11-21-session-start-deadline.md) | Session start: a deadline below the hook timeout | accepted | complete | [#69](https://github.com/writfold-dev/writfold/issues/69) |
| [2026-11-21](2026-11-21-bounded-graph-reads.md) | Bound the edges a graph answer reads; a work limit never certifies completeness | accepted | complete | [#60](https://github.com/writfold-dev/writfold/issues/60) |
| [2026-11-21](2026-11-21-loops.md) | Loops: a plan step that repeats a template until its gates are met | accepted | complete | [#74](https://github.com/writfold-dev/writfold/issues/74) |
| [2026-11-21](2026-11-21-project-asset-catalogs.md) | Project asset catalogs as reviewable Git projections | accepted | complete | [#84](https://github.com/writfold-dev/writfold/issues/84), [#86](https://github.com/writfold-dev/writfold/issues/86) |
| [2026-11-21](2026-11-21-record-option-ownership.md) | Record options belong to the kind that consumes them | accepted | complete | [#85](https://github.com/writfold-dev/writfold/issues/85) |
| [2026-11-22](2026-11-22-release-speed.md) | Release speed: incremental claims, an early site check, a parallel unit gate, one release command | accepted | complete | [#91](https://github.com/writfold-dev/writfold/issues/91), [#92](https://github.com/writfold-dev/writfold/issues/92), [#93](https://github.com/writfold-dev/writfold/issues/93), [#94](https://github.com/writfold-dev/writfold/issues/94) |
| [2026-11-22](2026-11-22-split-translation.md) | Split translation: a mutant rebuilds only the proofs it can reach | accepted | complete | [#95](https://github.com/writfold-dev/writfold/issues/95) |
| [2026-11-22](2026-11-22-missing-history.md) | Missing history is not evidence of change | accepted | complete | [#90](https://github.com/writfold-dev/writfold/issues/90) |
| [2026-11-23](2026-11-23-run-list-gaps.md) | The run list names what it cannot list | accepted | complete | [#98](https://github.com/writfold-dev/writfold/issues/98) |
| [2026-11-24](2026-11-24-resume-context.md) | The handover packet: a bounded resume whose required rows cannot be silently dropped | accepted | complete | [#109](https://github.com/writfold-dev/writfold/issues/109) |
| [2026-11-25](2026-11-25-prose-rules-and-advice.md) | Prose rules for delivery drafts: rules as data, advice separate from blocking | accepted | complete | [#127](https://github.com/writfold-dev/writfold/issues/127), [#115](https://github.com/writfold-dev/writfold/issues/115) |
| [2026-11-25](2026-11-25-vale-for-documents.md) | Vale for documents: a built-in check with a pinned, bound profile | accepted | in-progress | [#117](https://github.com/writfold-dev/writfold/issues/117), [#131](https://github.com/writfold-dev/writfold/issues/131)–[#134](https://github.com/writfold-dev/writfold/issues/134) |
| [2026-11-24](2026-11-24-retained-gate-evidence.md) | Retained gate evidence: a gate keeps the files its checker wrote | accepted | complete | [#121](https://github.com/writfold-dev/writfold/issues/121) |
| [2026-11-25](2026-11-25-agent-vocabulary.md) | Composable tools, gates, skills and agent profiles: definitions, invocations and a report bound to one | accepted | partial | [#143](https://github.com/writfold-dev/writfold/issues/143)–[#149](https://github.com/writfold-dev/writfold/issues/149), [#151](https://github.com/writfold-dev/writfold/issues/151) |
| [2026-11-25](2026-11-25-evidence-governed-execution.md) | Evidence at each use site: what is already judged, and requirements at prepare | accepted | complete | [#153](https://github.com/writfold-dev/writfold/issues/153), [#154](https://github.com/writfold-dev/writfold/issues/154) |
| [2026-11-25](2026-11-25-kb-research-contracts.md) | KB research contracts: a scope gate, generated review units and finding disposition | accepted | partial | [#155](https://github.com/writfold-dev/writfold/issues/155), [#156](https://github.com/writfold-dev/writfold/issues/156), [#157](https://github.com/writfold-dev/writfold/issues/157) |
| [2026-11-26](2026-11-26-research-contracts-second-slice.md) | Research contracts, second slice: live reports, a binding that survives its step, a judged universe, unit-local staleness | accepted | complete | [#163](https://github.com/writfold-dev/writfold/issues/163)–[#168](https://github.com/writfold-dev/writfold/issues/168) |
| [2026-11-26](2026-11-26-pinned-agent-closures.md) | Pinned agent closures: an edited profile reads policy moved | accepted | complete | [#161](https://github.com/writfold-dev/writfold/issues/161) |
| [2026-11-27](2026-11-27-tauten-adoption.md) | tauten adoption: from a pilot to a gated step, and what stays Writfold's own | accepted | not-started | [#171](https://github.com/writfold-dev/writfold/issues/171)–[#177](https://github.com/writfold-dev/writfold/issues/177), [#99](https://github.com/writfold-dev/writfold/issues/99)–[#103](https://github.com/writfold-dev/writfold/issues/103) |
| [2026-11-27](2026-11-27-referents-remainders-sites.md) | Three laws the calculus lacks: a resolved referent, an accounted universe, a site with a neutral event and a posture | accepted | not-started | [#178](https://github.com/writfold-dev/writfold/issues/178), [#190](https://github.com/writfold-dev/writfold/issues/190), [#191](https://github.com/writfold-dev/writfold/issues/191) |
| [2026-11-27](2026-11-27-citation-target.md) | The commit a citation is judged against is a decision: the remote that names the cited repository, never one behind the cited commit | accepted | not-started | [#179](https://github.com/writfold-dev/writfold/issues/179), [#180](https://github.com/writfold-dev/writfold/issues/180) |
| [2026-11-27](2026-11-27-run-directory-accounting.md) | A handover accounts for every file in the run directory | accepted | not-started | [#181](https://github.com/writfold-dev/writfold/issues/181) |
| [2026-11-27](2026-11-27-partial-clone-identity-first.md) | Partial clones: decide by blob identity first, name the blobs a verdict still needs, fetch them only when asked | accepted | not-started | [#182](https://github.com/writfold-dev/writfold/issues/182), [#192](https://github.com/writfold-dev/writfold/issues/192) |
| [2026-11-27](2026-11-27-guard-check-and-posture.md) | The guard site without a harness: `guard check`, a neutral event, harness bindings and a declared posture | accepted | not-started | [#183](https://github.com/writfold-dev/writfold/issues/183), [#184](https://github.com/writfold-dev/writfold/issues/184) |
| [2026-11-27](2026-11-27-session-ledger.md) | A session ledger: what the harness reported, `saw:` requirements over it, and actors on rules | accepted | not-started | [#186](https://github.com/writfold-dev/writfold/issues/186) |
| [2026-11-27](2026-11-27-outbound-remainder.md) | Outbound completeness: an unclassified call is counted, a positional file is a body, a deliverable may have parts | accepted | not-started | [#187](https://github.com/writfold-dev/writfold/issues/187), [#188](https://github.com/writfold-dev/writfold/issues/188) |
| [2026-11-27](2026-11-27-claims-in-text.md) | Claims inside text: a minimum number of citations, and every block of a draft accounted for | accepted | not-started | [#189](https://github.com/writfold-dev/writfold/issues/189) |
