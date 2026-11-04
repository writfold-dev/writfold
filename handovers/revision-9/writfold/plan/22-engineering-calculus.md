# Engineering calculus: conservative operational extension

**Status: proposed specification and proof obligations, not proved theorems.** Existing kctx semantic 2.0.0 and T01-T22 remain unchanged. The retained 353-declaration registry is historical evidence for those recorded assumptions only. New obligations E01-E12 are separately inventoried in `registries/engineering-theorems.json`; none is counted as an existing Lean declaration.

## Semantic domains

Let K be an existing authenticated knowledge state with named contexts and witness checks. Let P be a finite admitted procedure catalog, including pinned skill packages, typed IR, effect ceilings and gate floors. Let J map operational StreamIds to validated finite event prefixes. Let O be shared resource ownership with generations. Let A be current authority/policy observations. Let Z be immutable retained objects and their typed identities. Let Q be projection metadata and reader cuts. Let L be candidate/evaluation state reconstructed from learning streams. Let W be modeled external world state, accessible only through qualified observations/effect contracts.

WellFormed requires catalog P to resolve from exact authenticated corpus release/package pins, including any explicitly retained older activation pins. It is not an independently mutable source of procedure authority. Historical transition validation uses retained historical premises; current guards govern new continuation.

The operational state is Sigma = (K, P, J, O, A, Z, Q, L, W). K and W are not replaced by graph-shaped JSON. Omnigraph provides qualified representations and retrieval; the mathematical transition rules do not depend on GQ syntax or ANN ranking. Hashes denote identity under explicit collision-resistance/codec assumptions, not semantic truth.

Ground types include nominal identities, finite maps/sequences, typed capability sets, expected revisions, bounded budgets, independent evidence dimensions, and closed outcomes. The first Lean model can use finite domains with arbitrary mathematical integers where existing semantics requires them; Rust conversions must establish declared bounds. Do not impose u64 silently on the inherited calculus.

## Judgments

| Judgment | Meaning and evidence boundary |
| --- | --- |
| K, c |- usable(claim) | Existing context-specific usability judgment; no new pooling rule |
| Gamma |- package : AdmissiblePackage | Exact payload/lock closure, compatible loader, validated paths/exports and admitted profile |
| Gamma |- p : Proc(I,O,Eff,Floor) | Well-typed finite procedure; calls/branches/joins compatible; no unauthorized effect or weakened gate floor |
| Sigma |- activate(p,i) => activation | Current selection/grant, retained closure, typed inputs and exact run binding |
| Sigma |- prepare(a) => intent | Enabled step, evidence predicates, authority, budget, resource ownership and exact subject binding |
| Sigma, intent |- dispatch => effect | Durable retained intent; current guards rechecked; adapter contract supplies actual conditional-effect semantics |
| Sigma, observation |- reconcile(e) => outcome | Observation authenticates exact effect/subject and justifies Confirmed/Failed/Partial; otherwise Unknown remains |
| Sigma |- return(child,result) => parent | Matching invocation/binding plus checked postconditions and required evidence |
| Sigma |- propose(delta,E) => candidate | Retained observations with explicit scope; no claim of truth/admission |
| Sigma |- evaluate(candidate,protocol) => evaluation | Frozen inputs/rubric, qualified evaluator and complete recorded outcomes |
| Sigma |- admit(candidate) => operation | Existing knowledge admission plus procedure/policy/evaluation conditions, expected predecessor and authorized exact delta |

Gamma is the relevant typed environment: immutable package/adapter contracts, schemas and registered predicate definitions. Arbitrary strings, Markdown assertions, script exit codes and agent judgments cannot populate a trusted premise without the declared validation/assumption boundary.

## Representative rules

PREPARE requires: WellFormed(Sigma), enabled node and matching run/activation; accepted unchanged design where required; subject/dependency equality; complete applicable evidence for the required gate set; Usable claims under their own contexts; current authority covering a; owned resource set at matching generation; remaining budget; no unresolved conflicting effect. It produces a bounded typed intent and retains its input closure before the stream head advances. It does not execute the effect.

DISPATCH first commits a unique dispatch claim and stamp to the selected ledger head before emitting the external command. Only the live claim owner may issue it once. A persisted claim without a conclusive observation becomes Unknown after recovery, including a crash before IO. Qualified retry gets a new dispatch claim while retaining the original stable logical effect identity. DISPATCH requires the exact prepared intent selected by the current expected head, durable closure, still-applicable grant and resource generation, plus current external preconditions. An adapter with provider CAS binds the write to those expected revisions. An adapter lacking such a primitive cannot receive an atomic check-and-write theorem by assumption; expose a weaker operation or refuse the required guarantee. The world transition is nondeterministic within the adapter contract. A lost reply transitions to Unknown.

CALL requires compatible input types, established callee preconditions, an admitted pinned export, available budgets and effective capability subset. ChildEff is bounded by RunGrant intersect ParentDelegation intersect CalleeCeiling; Floor includes inherited required gates. RETURN requires output evidence for the exact invocation and inputs. Cancellation/stale completion cannot satisfy a Join.

REUSE requires matching typed fingerprints and subject, complete dependency capture, applicable profile/scope, retained authentic evidence, current freshness/correction/policy conditions and the declared reuse rule. It yields a reused-evidence record referencing the original receipt, never a fictitious new execution or observation timestamp.

PROMOTE requires an evaluated exact candidate, applicable approval, valid existing knowledge witnesses/contexts for claims, well-formed procedure changes preserving the policy floor, unchanged expected predecessor and retained evidence closure. It prepares an ordinary corpus admission. Only the existing completed Git CAS makes the new release selected. Activity acknowledgement is a later observation.

## Closed operational actions

Specify Init plus Resolve, Activate, LoadResource, SelectRoute, Call, Prepare, Reserve, ClaimDispatch, IssueEffect, Observe, Reconcile, Return, Rebind, Reopen, Cancel, Quiesce, Propose, Evaluate, Review, PrepareAdmission, PublishAdmission, AcknowledgeAdmission, Project, AcquireReadCut, ReleaseReadCut, Crash, Recover and Retire. Every action has frame conditions, explicit refusal behavior and an IO mapping. The model also includes explicit environment actions: AdvanceTime, RevokeGrant, ChangePolicy, WithdrawProcedure, CorrectKnowledge, AdvanceUpstream, ChangeProviderState, CompleteExternalEffect, LoseReply and TerminateProcess. Each has an explicit allowed relation and frame conditions; observations of W are distinct from W itself. External completion may occur after response loss and before reconciliation. Do not prove revocation/freshness/compatibility properties in a frozen environment. Rejection leaves authoritative state unchanged except explicitly permitted diagnostic/observation records; those records grant no authority.

Ledger append prepares immutable objects then conditionally advances a stream head. Ownership changes coordinate across streams. Reserve + Dispatch composition must establish that an admitted child cannot outlive a released reservation. Crash/recovery can leave Unknown effects; it cannot invent outcomes. Projection actions never modify K, authority or operational heads. Read cuts preserve their pinned interpretation while newer heads advance.

## Theorems to prove

| ID | Proposed statement | Essential negative calibration |
| --- | --- | --- |
| E01 | Conservative extension: new actions cannot derive additional legacy knowledge conclusions except through existing admitted operations | Treat a skill instruction as a witness |
| E02 | Type/representation preservation: Init and every admitted action preserve WellFormed | Cross namespace cast / malformed revision |
| E03 | Capability non-amplification and gate-floor preservation across nested calls and learning | Child grants itself publish; learned procedure drops a required review |
| E04 | Subject/dependency integrity of checks, reuse and returns | Reuse a pass after SDK edit; return old child into new activation |
| E05 | Operational history integrity and expected-head conflict safety | Two appends from same predecessor both selected |
| E06 | Exclusive conflicting resource use under qualified fencing/quiescence assumptions | Two runs share a writable worktree; release before child exit |
| E07 | Outcome honesty: no Confirmed/Failed without the adapter evidence needed for that classification | Timeout implies failure; blind replay |
| E08 | Publication closure and promotion integrity | Admit lesson whose evidence exists only in prunable activity |
| E09 | Projection non-authority and exact-cut interpretation | Lagging row authorizes merge; pagination advances HEAD |
| E10 | Version-pinned composition and bounded resource traversal | Moving dependency alias; recursive call escapes budget |
| E11 | Declared compatibility preserved by each permitted cross-repo transition, assuming sound compatibility observations | Merge consumer before required producer |
| E12 | Revocation/correction-aware continuation and explicit migration | Expired grant or withdrawn procedure remains executable because digest matches |

Safety proofs are conditional on explicitly stated premises: existing EvidenceSound where used, authenticated approvals, codec/hash properties, qualified storage/conditional operations, truthful adapter observations within their contracts, enforced capability boundary and cooperating-user/local-owner assumptions. They do not prove arbitrary source truth, arbitrary Python/shell correctness, causal learning improvement, complete test adequacy or an infallible LLM.

## Refinement and composition plan

Lean defines closed judgments and executable finite reference functions with non-vacuous positive/negative examples. No sorry/admit/custom axioms beyond the reviewed baseline and explicitly inventoried environment assumptions. Verus V16-V21 verify the actual shipping pure bodies and safe conversion callers. Shared vectors support conformance but are not a cross-prover equivalence theorem. Any stronger refinement claim needs a proved bridge.

TLA+ models: M12 EngineeringLedgerOwnership; M13 EffectsAndCrossRepo; M14 SkillComposition; M15 LearningPromotion; M16 ProjectionCutsRetention. Each defines TypeOK, a strengthening invariant, Init, all Next actions, crash/restart and frame/stuttering conditions. TLAPS proves Init => Inv and Inv /\ Action => Inv' for each action, plus the named bridges below. TLC explores finite concurrent/crash configurations and records bounds/state counts. Liveness stays conditional on fair scheduling, eventual adapter settlement, available authority and stable enough dependencies; no general TLAPS temporal claim is made.

Mandatory bridges: M12<->M13 (heads, reservations and effect identity); M12<->M14 (invocations, budgets and teardown); M15<->M01/M02 (exact admission and lost acknowledgement); M15<->M03/M11 (promotion evidence and epoch retention); M16<->M03/M10/M11 (pins, generations, cuts and GC); M13<->M09 (design binding/reopen and receipt invalidation); M14<->M07/M08 (shared supervision mechanics, separate permissions). Bridge invariants and production symbols must be registered; importing model names is insufficient.

Adversarial minimum: two runs, overlapping resource sets, two procedure revisions, two corpus releases, two schema epochs, two competing promotions, a lagging projector, cancellation and crashes around every durable/dispatch boundary. Mutants must violate the intended semantic invariant; syntax errors/solver timeouts do not count. Proof recipes and receipts follow plan/03. All E/V/M obligations added in this revision remain `not_run`.

E11 scope: permitted landing transitions preserve the declared compatibility relation only under its stated environment relation and sound observation/CAS assumptions. Arbitrary independent upstream/deployment changes can violate actual-world compatibility; the model must represent them, invalidate affected observed bindings when detected, and block unsafe continuation. It must not claim Writfold prevents every uncontrolled external change. E12 forbids new dispatch under known inapplicable authority and requires the qualified authorization-at-effect boundary; instantaneous worldwide revocation is not assumed.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.
