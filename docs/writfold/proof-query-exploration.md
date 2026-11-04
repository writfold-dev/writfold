# External proof language: exploration handover

Prepared on 4 November 2026 for a separate session with Tarek. This is a design
handover, not an admitted Writfold implementation contract. **Continue the
exploration here; do not silently activate a new prover, language, worker,
admission rule or dependency.**

## User intent and parallel ownership

Tarek wants **Weft** to ship alongside Writfold as an agent-friendly language
frontend. His latest direction is: settle the syntax and required Writfold atoms
(opcodes), let Weft perform lowering and semantic passes, then hand the resulting
typed recipe to Writfold for validated execution. A REPL is desirable. A general
bytecode VM is not required; direct typed operations and bounded recipes should
reuse Writfold's procedure/execution services rather than create a second engine.
Syntax, atom set, recipe format and exact trust boundaries remain unsettled.

Tarek approved **features; separate proof-policy decision** for revision 9.
The citation integrity/compact reader, Run Watch/session handoff and authorized
corpus discovery are admitted for planning through
[the scoped supplement](revision-9-features.md), not wholesale v9 adoption.
The implementation session is now tasked with the citation calculus and
production reader first, then execution-ledger and skill/procedure work in the
existing dependency order. Do not redirect that work into this speculative
language or edit its in-flight files from this exploration session.

Current implementation/admission authority remains
[`handovers/revision-5/writfold/`](../../handovers/revision-5/writfold/README.md).
The blanket requirement for a Lean-kernel instance certificate for every
proposal (v9 plan 25, CIT07/V23 and associated profiles) remains **deferred**.
Existing proofs, checked witnesses, review, grants, freshness, conditional heads
and publication checks are not relaxed. The external solver is not an alternate
admission route. The known P01O/WF24 native storage blocker remains authoritative;
do not rerun it merely to confirm it or invent a fallback.

## Starting material and exact identities

The checkpoint through `a26a7be` was pushed to private `origin/main`: three earlier
commits plus the revision-5 engineering calculus/primitives checkpoint. A fresh
baseline check and 192/192 workspace test obligations passed before that push.
Later planning work is separate; inspect the actual current tree before edits.
A checkpoint does not renew historical whole-tree qualification receipts.

| Input | Identity |
| --- | --- |
| `~/Downloads/writfold-rust-handover-version-9.zip` | 46,837,240 bytes; SHA256 `1efc4222a67380ae969ba3012b63e1b55c045f09310cdb61e467d7dcab39574d` |
| Retained v9 bundle | `handovers/revision-9/writfold/`; 359 manifest members, plus manifest |
| V9 manifest | SHA256 `a49d90be040146c6cc80b36e61667aadcb00d122aab7134807d504497bb3221c` |
| `~/Downloads/writfold-proof-query-demo.zip` | 97,960 bytes; SHA256 `fb9171632017f8eb4153239e4ebb4699d29cd3947ea7e4d6d54e22539117d614` |
| Demo request | SHA256 `eafaadbcd4b440230772033367c7d937efc9e6855bd287f30f8217326b493caf` |
| Shared duplicate-line fixture | SHA256 `667752861bf3c1887c2f6d25c13b74e14398bf18a896a56cf20b031ef7b1ecb9` |

The proof demo remains outside the repository. Its archive root is
`writfold-proof-query-demo/`. Start with `README.md`, `request.json`, `replay.py`,
`pico-run.c`, `receipt/challenge.cnf`, `receipt/certificate.lrat` and
`evidence/duplicate-line.json`. The bundled C tools and their licenses are exact
review inputs, not admitted production dependencies.

The investigation report is
`~/.local/share/writfold/evidence/handover-review-20261104-001.json`.
Its hash and scope are recorded in
[`qualification/revision-9-feature-adoption.json`](../../qualification/revision-9-feature-adoption.json).
It retains successful commands and failed CLI/isolation attempts. Execution used
the operator's existing storage admission and Compose image
`writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6`,
with no network, read-only source mounts and a disposable executable temporary
filesystem. Ambient host compilers were not used for the successful replay.

## What actually ran

The demo compares exact UTF-8 bytes between LF delimiters, including indentation.
Let A mean target line 2 matches the historical line and B mean target line 5
matches. Both observations hold. The candidate claim is at most one match:
`not A or not B`.

1. The adapter derives unit clauses A and B from digest-pinned fixture bytes,
   without consulting the fixture's expected answer.
2. PicoSAT finds a model for the premises alone. The replay directly checks that
   assignment against every input clause.
3. Adding the claim creates `A and B and (not A or not B)`, which is UNSAT.
4. PicoSAT emits a trace, DRAT-trim translates it to LRAT, and LRAT-TRIM checks
   both the new proof and the supplied certificate against reconstructed CNF.
5. LRAT-TRIM rejects a missing-antecedent proof. Changed evidence fails its old
   digest pin. Rebinding that changed input produces a checked SAT model, and
   the old certificate is rejected on the changed formula.
6. The comparison-only `lrat-check.c` accepts **both invalid controls**, including
   the certificate on the satisfiable formula. It is excluded from acceptance.
   This is an observed qualification failure for that exact checker, not a claim
   that all LRAT checkers are interchangeable or that LRAT itself is unsound.

Successful replay hashes: CNF
`3d47848ba573b90cf78780efbcd7ed77a3f512cd3d63b2923ff20c741651c498`,
supplied proof `cde0fe18c044ca718524443e29d8359179e5593d6a92eceea960b5cfb4f9eb00`.
The result refutes the at-most-one proposition. It does **not** identify the
historical successor, establish program meaning, invoke Writfold, prove the
Python adapter correct, authenticate a live request or authorize a mutation.

Separately, the supplied discovery `CaseDiscovery.lean` checked with the current
Lean 4.21.0 toolchain. Its eight printed declarations used no axioms or only
`propext`. Those are abstract eligibility/read-erasure/projection facts, not a
proof of native retrieval, privacy or a new external language. Supplemental
Metamath checks are not a production dependency or a substitute for Lean.

## Working direction and open execution contract

Keep language elaboration and logical semantics outside Writfold. Weft lowers
to a versioned typed recipe; Writfold independently validates its types, allowed
operations, pinned inputs, budgets and current authority before execution.
Prefer extending the existing checked procedure/execution contract over adding
a parallel interpreter or scheduler. Writfold hosts exact scoped evidence,
owned child processes and immutable artifacts; proof search and certificate
checking may run in separately qualified external processes. Compilation and
successful proof checking cannot mint capabilities or bypass admission.

```mermaid
flowchart LR
    S[Weft language or REPL] --> I[Semantic passes and typed recipe]
    I --> W[Writfold validation and procedure execution]
    W --> E[Exact scoped evidence atoms]
    W --> P[Admitted external proof search]
    P --> C[Independent certificate checking]
    E --> C
    C --> R[Scoped conclusion and dependencies]
    R --> A[Separate current Writfold admission]
```

A checked result is evidence for a proposition under explicit premises. It is not
permission to execute a plan, evidence that a remote fact is true, or automatic
admission of a useful lesson. Read-only query access and effect authority remain
separate even when one external process can request both through different
capabilities.

The important distinction is **reasoning semantics versus execution services**.
Calling Writfold a capability/evidence host is precise. Calling it the theorem
VM risks moving the language's logic into the product core unnecessarily.

## Initial IR candidates

These are proposed semantic operations, not existing command names, frozen
opcodes or an instruction to implement a new runtime. Keep a small typed
representation with named immutable values and explicit dependencies before
choosing bytecode or surface syntax.

| Operation | Intended result | Owner and boundary |
| --- | --- | --- |
| `open_scope` | Exact named context/release/read-cut binding | Writfold resolves configured realm and authorization; no ambient latest context. |
| `resolve` | Exact evidence binding and retained representation, or explicit refusal/unavailability | Writfold evidence interface; a compact alias is expanded to its full durable binding. |
| `observe` | Typed observation from exact inputs under a registered interpretation | External qualified interpretation using captured evidence; no arbitrary prose-to-truth conversion. |
| `assume` | Named hypothesis in the query environment | External logic only; a user-supplied assumption is not an observed fact or corpus write. |
| `check` | Scoped logical outcome plus checkable proof/models and dependencies | External lowering/search/checker; no state mutation or granted capability. |
| `explain` | Projection of the actual checked result and premise dependencies | Presentation only; generated prose cannot strengthen the result. |

Some instructions map to typed Writfold requests. Others are local environment
operations or external solver work. Do not mechanically translate every opcode
into a Writfold shell command. A command transport can call the same typed service
as RPC; use structured data and argv, never concatenate evidence into shell code.
Current Writfold production APIs are incomplete: these rows describe the target
boundary, not available end-user commands.

`record`, `admit` and `apply` are deliberately not implicit consequences of
`check`. Decide separately whether the query tool may request artifact retention;
authoritative admission and real effects keep their own existing gates. Discovery
can suggest subjects but must not supply an exhaustive quantifier domain merely
because it returned no additional hits.

## Logical outcomes and errors

For premises P and proposition Q, check premise consistency before interpreting a
classical entailment/refutation result. Never turn inconsistent evidence into a
useful universal conclusion through explosion.

| Outcome | Required evidence |
| --- | --- |
| Entailed | P is consistent and `P and not Q` has a valid UNSAT certificate. |
| Refuted | P is consistent and `P and Q` has a valid UNSAT certificate. |
| Undetermined in this theory | Valid models of both `P and Q` and `P and not Q`. |
| Inconsistent premises | Valid UNSAT certificate for P itself; no normal entailment/admission conclusion. |
| Unknown | Required search/check incomplete, unsupported or resource-exhausted; no logical verdict inferred. |

Exact supported-theory and model-checking obligations need specification; the
SAT definition above is the starting case, not a universal SMT/prover contract.
Invalid proof, malformed input, unauthorized access and unavailable evidence
should remain typed execution/verification failures, not logical refutations.
Missing observations stay unknown rather than silently becoming false. Surface
explicit assumptions in explanations and downstream reuse, including assumptions
that were not needed by the final proof when that distinction can be checked.

## The hard proof obligations

1. **Evidence to premises:** register interpretation semantics, exact source pins,
   selector units, parameters and dependency closure. A comment saying a patch
   is safe cannot directly produce `Safe(patch)`. Recorded truth is distinct from
   current applicability and actual external state.
2. **Language to IR:** forbid ambiguous binding, mixed units, implicit context
   changes and silent coercion from observation/assumption into authority.
   Separate a guessed hypothesis from a validated observation structurally.
3. **IR to backend:** establish the appropriate soundness/reflection relation for
   each supported theory and polarity. A valid proof of the wrong CNF says
   nothing about the requested proposition. Record lowering identities; a digest
   binds bytes but does not prove their semantics.
4. **Certificate checker:** qualify the exact checker source/executable, supported
   proof dialect and invocation. Keep genuine invalid-proof and satisfiable
   controls. Two implementations or a successful status string alone do not
   establish correctness. Search engines can remain untrusted when their output
   is checked through a sound accepted path.
5. **Result use:** bind query, evidence, assumptions, context, intended use,
   interpretation/lowering/checker profiles and validity conditions. Recheck live
   authority/predecessor where any later admission happens. No cross-context
   pooling or silent substitution of newer evidence.

## Recipe interpreter and REPL

Tarek's latest direction places recipe execution in Writfold and the frontend/
REPL in Weft. Use one recipe semantics for batch and interactive submissions,
with the same typed primitives as direct operations. Begin with a bounded
dataflow recipe; general loops, arbitrary native code and a bytecode format
need demonstrated demand. The contract is not yet frozen or implemented.

A successful REPL cell introduces a new immutable value into an explicit query
environment. Failed/interrupted cells must not leave a successful binding. A new
scope or refreshed observation produces a new environment/version rather than
quietly changing earlier values. Rebinding a convenient name does not rewrite
previous derivations. Cancellation follows owned-child lifetime rules; stopping
a UI does not magically establish that a worker has stopped.

Export must yield replayable IR plus exact inputs/profile identities and retained
result artifacts, not just terminal text. Distinguish replay of captured results
from rerunning a solver, which can choose a different model/proof or exhaust its
budget. Rechecking an old conclusion and observing the current world are also
different commands. Completion/explanation should expose that distinction.

Illustrative syntax discussed, **not a frozen language**:

```text
use context checkout at R42
old = resolve citation c7
a = observe exact_bytes(old, target.line(2))
b = observe exact_bytes(old, target.line(5))
check at_most_one(a, b)
explain
```

This sketch leaves `target` binding and line-to-byte conversion implicit for
readability; a concrete proposal must make both explicit and qualified. Export
must contain full durable bindings, not a dependence on a transient `c7` alias.
Do not mistake this snippet for a runnable interface.

**Working name:** Tarek is now using Weft for the companion frontend. Earlier
`writfolds` was floated for an opcode executor, not selected as a second product.
No name availability, registration or installed command is claimed. Keep the
semantic proposal independent of eventual packaging and naming decisions.

## Relationship to v9 plan 25

V9's blanket rule is stronger than having a proved admission predicate applied by
qualified Rust code: it additionally requires a Lean-kernel-checked instance for
each distinct authoritative proposal, potentially on a separate trusted worker.
It brings worker availability, trust-root/key lifecycle, exact certificate
invalidation, proof provenance and predecessor-binding concerns.

That rule is deferred, not rejected and not silently weakened. The external
language may eventually produce useful evidence or certificates for an admitted
profile, but a SAT result does not automatically satisfy a Lean instance rule.
Keep exploratory query checking, reusable rule qualification and authoritative
proposal admission as separate decisions. Do not resolve their relationship by
assuming that one prover's certificate is another prover's refinement theorem.

## Concrete agenda for the next session

1. Pick two or three real questions beyond duplicate lines: exact contract
   compatibility, bounded dependency/permission checks, or a counterexample to a
   proposed procedure precondition. Identify what evidence can actually justify
   their premises. Do not promise arbitrary natural-language truth checking.
2. Freeze a small typed query/IR contract on paper. Specify scopes, evidence,
   observation results, hypotheses, propositions, theory profile, bounds and
   every result/error variant. Decide which operations are Writfold requests
   versus external computation. Distinguish `check` from effectful recipes.
3. Work through entailment, refutation, two-sided models, inconsistent premises
   and unknown with concrete examples. Include wrong polarity, a wrong lowering,
   missing evidence, stale/revoked scope and an incomplete search domain.
4. Specify a result packet: exact query/IR and evidence identities, claim/polarity,
   interpretations/lowering/checker, assumptions and validity, consistency proof,
   proof/model artifacts, checked dependency closure and verification failures.
5. Specify the Writfold recipe-execution boundary using the existing skill/
   procedure machinery where possible. Keep one semantics for direct typed
   operations, batch recipes and REPL submissions, with Weft owning elaboration.
   Evaluate ergonomics after the semantics are concrete, not by accumulating
   opcodes or building a parallel VM first.
6. Present a reviewed proposal with non-goals, trusted computing base, proof
   obligations, adversarial acceptance cases and integration/migration impact.
   Seek an explicit adoption decision before implementation. Retain all failed
   experiments; no mock solver/checker, guessed authority or synthetic receipt
   may be presented as actual proof integration.

The next session should brainstorm with Tarek, not merely implement this sketch.
The user is trying to untangle a vague idea into a solid proposal. The IR,
interpreter boundary, first supported theory, worker trust, outcome names and
surface language are all still open to revision.
