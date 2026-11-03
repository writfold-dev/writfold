# Rust semantic core

The IO-free `writfold-kernel` core implements the retained semantic-2.0.0
checker, mathematical time rules, and whole-candidate invariants. The Python
and Lean sources remain unchanged reference material. This implementation is
not a claim that executable Rust has received Verus qualification.

## Source mapping

| Reference module/functions | Rust owner |
| --- | --- |
| `semantics/time.py`: `fresh_at`, `assess_fresh` | `time` (same public names) |
| `semantics/checker.py`: `context_matches`, `source_selected`, `inventory_content`, `retracted`, `dependencies_union`, `correction_dependency`, `check`, `accepted`, `supported`, `same_context` | `checker` (same public names) |
| `semantics/checker.py`: `_check`, all six witness branches | private `checker::evaluation::{evaluate,evaluate_valid,attest,enumerate}` |
| `semantics/checker.py`: `reusable`, `witness_freshness`, `policy_allows`, `assess`, `usable_for`, `assessment_gaps` | `checker` exports from private `assessment` |
| `semantics/wellformed.py`: `nonempty`, `natural`, `ttl_wf`, `scope_wf`, `formula_wf`, `statement_wf`, `context_wf`, `witness_wf`, `dependency_wf`, `well_formed` | `wellformed` (same public names) |
| Whole-candidate attestation, enumeration, correction and supersession-cycle checks | `wellformed::{rows,graph}` |

The domain's nominal values eliminate Python runtime type confusion. They do
not turn candidate records into evidence or grant production authorization.
The existing model approval wrappers remain reference assumptions.

## Public calling convention

All KB/context/reference/witness/receipt inputs are borrowed. `supported`
receives its final `Polarity` by value. `check` returns `Option<Checked>`;
`accepted<'a>(&'a KB, &Context, &ClaimRef)` returns
`Vec<(&'a Witness, Checked)>`. Witnesses in accepted results belong to the KB;
only retained checked outputs own their dependency records.

`assess` returns `Assessment`, and `assessment_gaps` returns ordered
`Vec<String>` diagnostics. `dependencies_union` takes two dependency slices;
`witness_freshness` takes a context and dependency slice. `inventory_content`
takes a string slice and returns `ContentId`. `correction_dependency` returns
the domain's `Dependency::Corrections` variant. `policy_allows` takes the
seven Boolean arguments in reference order. Time functions take
`(&Ttl, &Integer, &Integer)`, returning `bool` and `Freshness` respectively.
Well-formedness helpers take borrowed domain values and return `bool`;
formula, statement, witness and dependency helpers take the KB second.

Internal evaluations borrow evidence, scopes, atomic conclusions and context
frames. Composed formula ownership moves upward rather than recursively
copying already-built conclusions. Boolean support queries short-circuit;
assessment reuses its selected accepted alternatives for freshness and
validation. No query duplicates the KB or materializes owned evidence merely
to compare a conclusion or receipt.

The recursive evaluator validates its immutable context frame once, then
checks each witness branch against that same KB/frame. Ghost-only content
views describe exact ordered footprints without adding fields to runtime
records. `checker::spec` exposes the mathematical contracts consumed by
admission and selection; it is not a second executable checker.

For workflow candidate evaluation, `usable_for_snapshot(&KB, &Context,
&SnapshotId, &ClaimRef)` and `check_matches_snapshot(&KB, &Context,
&SnapshotId, &Witness, &Statement)` use a concrete borrowed context frame.
Only the snapshot changes; policy, time, source order, intended use and target
remain the base context's values. `check_matches(&KB, &Context, &Witness,
&Statement)` offers the corresponding no-retained-output ordinary check.
These predicates do not establish registration or non-retraction of a target
claim on their own: workflow callers separately enforce the reference's
exact derivation registration and claim acceptance conditions.

## Preserved decisions

- Time arithmetic uses signed arbitrary-precision `Integer`, including
  direct evaluation of negative TTLs. Equality at expiry is fresh. Future
  observations and missing policy are unknown. Well-formed candidate
  contexts independently reject negative time or negative expiring TTL.
- Attestation, truth, conjunction, specialization, absence and universal
  witnesses retain their individual premises. Inventory completeness uses
  exact ordered members/universe equality, uniqueness, nonempty query and
  method, selected successful nonfuture observations, and byte-framed content.
  The framing counts UTF-8 bytes, not Unicode characters.
- Dependency unions preserve first-occurrence order and compare complete
  records. Correction dependencies retain only matching events effective at
  evaluation time, in KB order. Context dependencies normalize only `now` to
  zero. A receipt requires exact witness and ordered dependency equality,
  exact context except time, a passing result, and inclusive validity bounds.
- Retraction removes dependent derivations without erasing independent ones.
  Dispute and contrary support contest claims; supersession does not transfer
  validation. Whole-candidate validation detects supersession cycles without
  applying evaluation-time filtering to the history.
- Positive and negative support, applicability, freshness, validation,
  contestation and availability remain independent. Any fresh accepted
  alternative makes freshness fresh; all-stale alternatives make it stale;
  otherwise it is unknown. Within one witness stale outranks unknown.
  Policy applies to the selected claim's polarity, including negative claims.
- Recursive formula/witness operations do not invent depth-based semantic
  refusals. Resource bounds belong to the wire admission profile.

## Tests and remaining assurance

`crates/writfold-kernel/tests/core.rs` contains behavioral regressions for
inclusive/future/signed/unbounded freshness, independent witness revocation,
exact receipt ordering and context matching, borrowed snapshot evaluation,
contradictory evidence, independent assessment dimensions, and UTF-8 inventory
framing. The current P02 campaign also re-executed complete-response comparison
against Lean and Python on all 334 retained scenarios with zero mismatches.

The sealed P03 integration in
`~/.local/share/writfold/qualification/p03-kernel-010/native-smoke.json`
compiled and executed the static musl `writfold-ref` with `PATH=/nonexistent`.
All 335 responses (handshake plus 334 scenarios) matched both retained
Lean and Python responses field-for-field. This exercised the composed
borrowed checker, assessment, receipt-reuse, well-formedness, admission and
selection implementations. It reused retained oracle outputs; it was not a
fresh three-host gate, whole-kernel proof, or P03 completion. Failed proof
and native-build attempts remain alongside the successful runtime record.
Inventory premise comparison traverses the borrowed conjunction/atomic tree
directly, preserving ordered members, exact predicates, empty arguments and
the terminal truth node without allocating a duplicate expected formula.

Remaining proof work is source-bound Verus qualification of these shipping
bodies: V02 finite-witness conclusions and transitive footprints; V03 exact
integer freshness and independent assessment/policy precedence; V04 ordered
input reuse and correction behavior. Qualification must cover the borrowed
context-frame projection, owned-output materialization, safe caller
preconditions and the arbitrary-precision arithmetic trust boundary. The
unchanged Lean calculus, behavioral regressions, and reference agreement do
not prove these Rust bodies or a cross-prover refinement bridge. No semantic
proof twin or excluded Writfold decision body is supplied.
