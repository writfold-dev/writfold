---
rfc: "2026-11-25-kb-research-contracts"
title: "KB research contracts and review coverage"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-25
updated: 2026-11-25
issue: https://github.com/writfold-dev/writfold/issues/155
milestone: null
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-25-evidence-governed-execution"
  - "https://github.com/writfold-dev/writfold/issues/154"
---

# RFC: KB research contracts and review coverage

“Check all related KB references for inconsistencies” combines scope discovery, actual review and a semantic judgment. Make each part visible and independently checkable, so a search call or summary cannot stand in for omitted work.

This owner-requested planning set is for later grooming. It applies [evidence-governed execution](2026-11-25-evidence-governed-execution.md), and is tracked by [#153](https://github.com/writfold-dev/writfold/issues/153), [scope #155](https://github.com/writfold-dev/writfold/issues/155), [coverage #156](https://github.com/writfold-dev/writfold/issues/156) and [fixture #157](https://github.com/writfold-dev/writfold/issues/157). Examples below are illustrative schemas, not accepted binary input.

## Proof obligations before design

New decisions belong in pure `writfold-core` kernels, proved over shipped Rust through Charon/Aeneas before wiring. Reuse #145 for the proof ledger and the companion RFC for evidence admission.

| Kernel | Inputs and outputs | Required property; falsifying mutant |
|---|---|---|
| Scope eligibility | Recipe, manifest, observed closure and current basis to complete/partial/unknown | Complete implies all required source/page/frontier obligations are satisfied; omit a required page |
| Scope freshness | Recipe/corpus membership basis and current basis to current/stale/unknown | New related members or edges invalidate applicability; check only returned blob hashes |
| Unit generation | Canonical finite manifest and selected unit rule to deterministic required units | All-pairs produces each unordered distinct pair once; delete a pair or duplicate one |
| Report eligibility | Unit, input versions, producing invocation and validated report to eligible/refused/unknown | Wrong inputs, invocation or evidence kind cannot substitute; accept the unit ID alone |
| Coverage | Complete scope/unit universe and eligible reports to covered/missing/unknown | Coverage requires an eligible result for every required unit; treat absent universe as empty |
| Finding disposition | Findings, dispositions and approved completion policy to allowed/refused/unknown | Unresolved/uncertain/excepted results are accepted only as policy allows; accept a worker-authored waiver |

Parsers, enumeration IO, Git/index snapshots, pagination, reference/span reads, request assembly and external observations stay tested. Formal completeness is conditional on the declared universe and faithful observations. It is not a theorem of global semantic recall or correct interpretation of prose. Model extraction of typed facts must retain model-judgment provenance even if a subsequent domain constraint check is deterministic.

Pass extraction drift, Lean builds, axiom audit and falsifying mutants before wiring decisions. Use fault/property tests for pagination, loss of a store, new members, stale indexes, publication/recheck races and cold-read diagnostics. Reuse actual write/protocol models; do not introduce a second scheduler.

## Worked contract: inspect related KB references for inconsistencies

### 1. Define a reference universe

“All related” must be given a finite operational meaning. For example:

- Approved stores: the project's KB and named documentation repositories.
- A pinned revision or declared observation basis for each store.
- Seeds: the change's subject IDs and directly affected records.
- Membership: declared relations such as `describes`, `depends-on`, `supersedes` and incoming/outgoing citations, traversed to closure within those stores.
- Eligibility: record kinds, inclusion/exclusion rules and access scope recorded in the contract.
- Every page and traversal frontier exhausted; inaccessible required sources and resource limits produce partial coverage.

Existing typed KB links are one possible input. A new semantic relation inferred by a model remains a judgment unless separately validated. Vector or keyword searches can add candidates, but top-k retrieval cannot establish that all semantically relevant material has been found.

The enumerator produces a scope manifest with source versions, the recipe identity, selected references and their digests, index/source agreement, omissions, excluded classes, pagination/traversal closure and completeness status. “Complete” means complete within this declared universe. Global semantic recall remains unknown.

### 2. Generate review units

Generate the required work from the manifest, rather than let the agent choose what counts as done. Examples include one review per document against a subject contract, or a comparison for every pair in a declared set of overlapping claims.

For a fixture with five documents and a requirement to compare every unordered pair, the enumerator generates ten stable review-unit IDs. If eight have valid reports, coverage is eight of ten. Merely calling `kb find` cannot discharge the other two.

Comparing every pair is quadratic and often unnecessary. A production contract may instead partition by declared subject/property and compare all units within each group. That changes the coverage claim and must be visible. Do not silently substitute a heuristic subset for an all-pairs promise.

Each unit binds its input versions and relevant spans. If a unit requires full documents but the packet only contains excerpts, the unit is not fully supplied. Large reviews may use several bounded invocations and an explicit aggregation contract; a summary is a derived, potentially lossy artifact.

### 3. Record attributed results

Each unit produces a structured report with its unit ID, input references, claim spans, disposition, explanation and producing invocation. Suggested dispositions are `conflict-found`, `no-conflict-found` and `uncertain`.

A structural checker verifies required fields, reference/span existence, input/version bindings, actual context delivery, unit coverage and permitted dispositions. It cannot establish that an arbitrary natural-language explanation is correct merely because the JSON is valid or every citation exists.

Use a deterministic contradiction checker where facts and constraints are actually formalized, such as incompatible values for the same exclusive configuration property. Extracting those facts from prose may itself be fallible and must retain that distinction. For broader prose comparisons, record an attributed model or human assessment. Independent review may improve reliability, but does not become a mathematical proof by adding a second model.

### 4. Reconcile and conclude

Every discovered conflict needs the contract's required treatment: resolve it, explicitly preserve it as an unresolved finding, or obtain an authorized exception. A research-only task may validly conclude with unresolved conflicts; an implementation or publication step may require their resolution. The policy decides this, not a universal “no conflicts” gate.

Expose several results instead of one misleading green check:

- Scope coverage: complete within recipe `R` at source basis `B`.
- Review coverage: every required unit has an eligible result.
- Judgment: reviewer found a conflict / no conflict / uncertainty.
- Findings: resolved / outstanding / excepted.

An empty result set needs a positive completeness witness and an explicit empty-scope policy. A failed search is not a successful empty search. “No conflicts found” remains a statement about a particular search and assessment.

### 5. Handle changes and newly related documents

Bind evidence to the query recipe and source inventory, not only to files already returned. Otherwise adding a newly related document leaves all old file hashes unchanged and incorrectly preserves “all reviewed”. Initially, pin the whole relevant corpus snapshot. Later optimize invalidation only with a sound scope-membership dependency analysis that catches insertions and new edges as well as changed content.

Preserve completed work as history. Invalidate eligibility for dependent current operations and create the necessary new work using existing plan operations. Do not silently rewrite historical done records. A historical review can remain valid for its old snapshot while being insufficient for a claim about the current one.

## Formal scope and coverage model

Let $R$ be the admitted scope recipe and $B$ its source/inventory basis. Its finite declared universe is $D(R,B)$. An enumerator returns a manifest $M$ and observation witness $w$. The scope kernel can accept `Complete(M,R,B,w)` only when all declared inventory/frontier obligations are discharged. Without a snapshot-capable source, the contract must explicitly weaken its claim or refuse snapshot completeness.

For a selected all-pairs unit rule:

$$U(M)=\{\{d_i,d_j\}\mid d_i,d_j\in M,\ i<j\},\qquad |U(M)|=\frac{|M|(|M|-1)}{2}.$$

Use canonical reference IDs and version-bound inputs, not display order, for stable unit identities. A membership change affects the universe even when every previously returned document is unchanged.

$$
Covered(M,E,B)\iff Complete(M,R,B,w)\land
\forall u\in U(M),\ \exists r\in E:\ Eligible(r,u,B).
$$

The empty-manifest case additionally requires the contract's explicit empty-scope policy. Missing inputs, inaccessible stores or search errors do not satisfy `Complete`. Negation such as “no units missing” is valid only over a certified finite inventory.

Keep four independent judgments:

$$J=(Scope, Coverage, Assessment, Disposition).$$

For example, `(complete, 10/10, conflict-found, outstanding)` can be a valid completed research task and still fail a publication policy requiring resolved findings. `(complete, 10/10, no-conflict-found, recorded)` is not a theorem that no contradiction exists. An uncertain but valid report can discharge procedural coverage while leaving a conclusive-assessment obligation unmet.

The approved contract determines which coordinates gate a particular operation. The agent cannot replace those requirements with its own single “done” flag.

## Illustrative `.writfold/` authoring

These TOML examples are proposed extensions, not accepted by today's binary. `.../proposal` schema identifiers deliberately distinguish them from shipped schemas. They show roles and bindings; the named predicates, scope recipe, schemas and adapters would require implementations and conformance fixtures.

```toml
# .writfold/skills/kb-consistency.toml
schema = "writfold-skill/proposal"
id = "kb-consistency"
version = 1
plan = "kb-consistency"
required_tools = ["kb.enumerate", "kb.read", "review.record"]
instructions = """
Compare the references in each supplied review unit. Identify conflicts,
cite the relevant spans, and preserve uncertainty in your disposition.
"""

[execution]
context_binding = "every-model-invocation"
required_scope = "kb-related-references"
```

```toml
# .writfold/plans/kb-consistency.toml
# Existing plan syntax; referenced gate definitions below are proposed.
schema = "writfold-plan/1"
name = "kb-consistency"
steps = [
  "enumerate: scope-complete",
  "inspect: review-units-covered",
  "reconcile: findings-dispositioned",
  "conclude: kb-review-contract",
]
```

```toml
# .writfold/gates/kb-review-contract.toml
schema = "writfold-gate/proposal"
id = "kb-review-contract"
version = 1
class = "derived"

[[requires]]
predicate = "context.skill-bound-to-producing-invocation"
skill = "kb-consistency"
artifacts = "review-reports"
issuer_role = "harness"

[[requires]]
predicate = "scope.complete"
manifest = "related-reference-manifest"
issuer_role = "enumerator"

[[requires]]
predicate = "review.units-covered"
manifest = "review-unit-manifest"
artifacts = "review-reports"
issuer_role = "structural-validator"

[[requires]]
predicate = "findings.dispositioned"
artifacts = "reconciliation-report"
issuer_role = "structural-validator"
```

Artifact names resolve to typed, step-bound outputs, never arbitrary paths supplied by the model at check time. The first predicate requires an eligible context receipt for each producing invocation represented in the report set. The last two establish coverage and recorded disposition, not semantic truth. Additional gates can require a particular assessor role or a deterministic domain checker where appropriate.

The execution block installs invocation admission requirements; it does not create another set of workflow steps. Entry validation uses the prepared request, while the concluding gate uses its later submission/result observations. This avoids the circular requirement of needing a dispatch receipt before dispatch.

The scope recipe belongs in the existing context configuration direction, with explicit stores, relation kinds, closure policy, units and partial/empty behavior. Do not quietly interpret arbitrary prose as executable gate logic. A policy authoring assistant may draft the structured contract once; retain a trace from each prose clause to its rule or to an explicit advisory judgment. The admitted policy remains fixed for the run unless an authorized revision is recorded.

### Explicit scope recipe

This additional proposal sketch shows the information the recipe must carry. `project-kb` resolves to an approved store binding; `<snapshot>` is an illustrative placeholder, not a valid commit pin. The initial contract pins the full relevant corpus, which safely catches newly related documents. Later selective invalidation needs its own sound membership-dependency argument.

```toml
# .writfold/context/kb-related-references.toml
schema = "writfold-research-scope/proposal"
id = "kb-related-references"
stores = ["project-kb"]
source_basis = { "project-kb" = "<snapshot>" }
seeds = "step.subjects"
relations = ["describes", "depends-on", "supersedes", "cites"]
direction = "both"
closure = "exhaustive-within-declared-stores"
on_partial = "refuse-complete"
on_empty = "require-explicit-allow-empty"
units = "all-unordered-pairs"
```

Relation names are selected contract vocabulary, not a claim that every named relation exists in today's KB schema. Unknown relations or unresolved store bindings must refuse rather than silently shrink the universe. A production recipe may deliberately use a different unit rule, with a correspondingly different coverage claim.

## Concrete acceptance fixture

Create synthetic documents A through E and a declared complete scope at basis B1. Generate AB, AC, AD, AE, BC, BD, BE, CD, CE and DE. Each report refers to its exact unit inputs and producing invocation. Give the fake reviewer valid reports for eight units, withholding BD and CE.

The expected diagnostic is: `cannot conclude: 8/10 review units covered; missing BD, CE`. Supplying the missing two permits the coverage check, but finding disposition remains a separate check. A correct hash without a protected context observation still fails the invocation-bound requirement.

Add related document F without changing A–E. The B1 result remains valid history; it cannot establish current scope completeness at B2. If the chosen rule remains all-pairs over six documents, the new manifest has fifteen units. Reuse prior reports only when their individual bindings and reuse policy permit it, and never reuse the old whole-scope completeness claim.

Include a schema-valid report with a deliberately wrong interpretation. Structural checks may accept its format and provenance; it must still be displayed as model assessment. Add a separate typed-fact fixture only if its formal vocabulary and extraction trust boundary are explicit.

## Execution, changes and recovery

The external driver schedules mandatory scope enumeration before review. It builds bounded review requests from generated units and obtains the invocation admission defined in the companion RFC. A model may propose extra references or findings; the driver validates them against a deliberate scope revision instead of silently widening or shrinking the admitted task.

Compaction/new attempts need current request evidence. Changed context or source snapshots require the applicable checks again. Historical completed plan steps are never silently rewritten; use existing plan operations to represent additional required work. Crash or lost dispatch observations remain unknown and require reconciliation.

The first slice uses a fixed synthetic corpus and a fake model. #155 owns scope, #156 owns coverage/disposition, #154 owns shared gate admission, and #157 owns the end-to-end fixture. Existing #148/#149 supply request and result evidence; #150/#64 supply adapter and receiver boundaries. #151 can present the example without reimplementing these pieces. There is no new search engine, workflow service or paid evaluation in this intake.

## Compatibility and open grooming decisions

Use `.writfold/` as the authored source. Preserve plain text for instructions and explicitly advisory assessments; do not translate arbitrary prose into executable policy without review. Choose the first supported scope/unit grammar, required-store behavior and authorized exception policy during grooming. The draft proposal schemas have no backward-compatibility obligation. Any future replacement of shipped behavior must be explicit in its implementation notes.

The research basis and trust assumptions are shared with [the evidence-governed execution RFC](2026-11-25-evidence-governed-execution.md#research-foundations-and-choices). No claim that a finite recipe finds every semantically relevant fact is proposed.

## Decision log

- 2026-11-25: Separated scope completeness, review coverage, semantic assessment and finding disposition at the owner's request. Added corpus-membership invalidation and a five-document/ten-unit fixture. The issue set is held for later grooming; no runtime change or new proof is claimed.
