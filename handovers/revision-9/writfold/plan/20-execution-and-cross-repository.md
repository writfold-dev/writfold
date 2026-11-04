# Durable execution across repositories

Revision 5 operational contract. The first executable vertical slice adapts the existing author-change workflow: inspect knowledge, prepare Worktrunk worktrees, plan and challenge, implement, test, review, refresh docs, publish PRs, address feedback, merge in compatibility order and record relevant findings. Rollout adapters are admitted separately. No real repository, PR or deployment is authorized merely by this handover.

## State and ownership

A Run record pins its procedure, accepted design, initial knowledge context, authority reference and task completion boundary. RunBindingRevision binds the current repo vector, worktrees, dependency/environment profiles and selected read set. Rebinding preserves history and invalidates affected receipts. Material design changes use the existing explicit reopen/supersede rules; do not replace design-v3 semantics.

Run lifecycle: Proposed -> Prepared -> Active -> Waiting/Blocked/Replanning -> Completed, with explicit Failed/Cancelled terminal outcomes. Completion is defined by the task: reviewed patch, opened PRs, merged code, released artifacts or operationally validated rollout. Those states are distinct. Each attempt/effect has its own lifecycle; a cancelled run may still retain unresolved external effects and cannot free their resources prematurely.

One coordinator serializes each stream head and owns shared resource reservations. ResourceKey is typed: repository ref, worktree root, provider object, release channel or deployment target. Two runs must not hold incompatible write reservations. Acquisition of a multi-resource set uses a canonical order or an atomic coordinator operation; waiting consumes a budget and cannot deadlock indefinitely under the declared terminal-state convention.

Local fencing requires both managed locks and actual access control/cooperating-client assumptions. An ownership generation checked only by the coordinator cannot stop an old unrestricted process writing a file. Do not transfer a workspace reservation until its old process tree is terminated/reaped or proven quiescent. If quiescence is unknown, retain ownership and block. External targets need provider CAS/idempotency or explicit reconciliation; a local lease does not fence a GitHub server.

## Step-by-step author-change procedure

1. **Intake:** bind objective, repos, completion boundary, standing grants, budget, procedure and context. User clarification is required only for material unresolved intent or authority.
2. **Inventory/sync:** inspect dirty work and ownership, verify RepoIds/remotes, fetch only under the relevant grant, resolve exact upstream bases, record baseline checks. No destructive reset of user work.
3. **Worktrees:** invoke an admitted Worktrunk adapter with a pinned tool/config/hook profile. Verify resultant repository, base, branch and contained path; supervise background hooks. Preserve existing external BuildKit configuration.
4. **Research/assumptions:** query pinned contexts and per-repository conventions. Track supported, refuted, unknown and stale assumptions with dependencies. Complete enumeration and contrary evidence have explicit coverage; no pooling of independent contexts.
5. **Design:** record alternatives, compatibility matrix, implementation plan, module responsibilities, tests, docs, landing/rollout order and recovery strategy. Challenge assumptions and generalization. Review findings require dispositions and evidence. Accept and bind exact targets.
6. **Implement:** use repository conventions, nominal types and small cohesive modules. Maintain the existing 400-line module/80-line function budgets and reviewed exceptions; splitting by line count alone is insufficient. Required code/proof/lint gates are defined before checking them.
7. **Validate/repair:** run relevant tests/proofs and cross-repository combinations. Bind every receipt to actual input bytes and environment. Distinguish infrastructure failure, test defect, implementation defect and invalid assumption. Retry only within the recorded policy/budget.
8. **Review/docs/PRs:** review behavior against design, refresh documentation and examples, publish exact branches under authority, link dependent PRs. Track reviewer findings and dispositions. Material changes reopen design. New pushes/rebases invalidate affected receipts and reviews.
9. **Merge/rollout:** revalidate live grants, provider state and required checks, then conditional operations in the declared order. Qualify the actual merge candidate/merge-group identity, not merely an earlier branch head. Distinguish merged code from deployed artifacts.
10. **Close/learn:** compare actual completion against task boundary, retain unresolved outcomes and follow-up obligations, release only quiescent resources, and evaluate learning candidates. Learning observations are captured throughout, not only at closure.

## Effect Adapter Protocol 1

An adapter descriptor binds code/runtime digests, operation schemas, capability ceilings, target identity rules, limits and reconciliation support. Runtime envelopes bind protocol version, session incarnation, request generation, EffectId, AttemptId, EffectIntentDigest, per-attempt DispatchStampDigest, expected run head, ownership generation and exact resource targets.

Commands are closed operations such as PrepareWorktree, RunCheck, PushRef, OpenPullRequest, ReadReview, MergePullRequest and ObserveDeployment. They are not arbitrary shell strings. Some adapters internally execute qualified scripts. A script cannot invent a validated receipt or act outside its delegated profile.

Prepare validates typed inputs, evidence, authority and resource ownership; retain the input/intent closure before dispatch. Immediately before crossing the effect boundary recheck relevant live guards. Use provider-enforced expected revisions wherever available. A local check followed by an unconditional remote write does not close a race.

Before any external IO, conditionally commit a unique DispatchClaim (AttemptId, stable intent, dispatch stamp and owner) to the ledger under stream/resource coordination. Only that live owner may issue the claim once. Duplicate deliveries cannot issue it again. A crash after claim persistence but before the physical call is still Unknown on recovery; the ledger cannot atomically prove whether the external call happened. Never issue IO first and record dispatch afterward. A new dispatch attempt for the same logical effect requires qualified provider idempotency or independent proof of no prior effect.

Effect lifecycle: Prepared -> DispatchClaimed -> Confirmed / Failed / Unknown. Unknown -> Confirmed / Failed after independent reconciliation, or remains Blocked. Failed means proven rejection/no relevant effect under that adapter's outcome contract. Partial effects are represented explicitly, not collapsed into Failed. Response loss, transport cancellation and process death do not prove non-application.

An EffectId identifies one logical external operation. Stable EffectIntentDigest binds its target, payload and remote preconditions. A separate DispatchStampDigest binds the current grant, ownership generation, expected run head and session/attempt. Restart may require a fresh dispatch stamp; it does not change the original operation identity. Adapter-supported idempotent retry may reuse it with identical input; changed input conflicts. Where the provider has no deduplication, do not promise exactly-once effects. Reconcile using exact provider object/ref identities; if existence is ambiguous, block. Opening a second PR is not an acceptable recovery from an unknown first creation.

Cancellation closes new admission and initiates descendant shutdown. Capacity and reservations remain charged until qualified completion/teardown. Stale session replies cannot advance state. Recovery reconstructs ledger state and observes external reality; it does not replay all Dispatch records. A restored old backup requires a new local incarnation and reconciliation before continuing.

## Cross-repository compatibility

RepoVector captures all participating repositories, bases, targets, workspace content manifests and artifact/dependency bindings. A compatibility relation classifies permitted intermediate combinations under explicit assumptions. The procedure plans a path through those combinations. Its abstract preservation theorem is conditional on the declared compatibility model; real behavior is qualified by contract/integration tests.

Example: an API, Rust SDK and CLI add opaque cursor pagination. Inspect old decoder behavior, test old/new consumer/producer combinations, preserve old API behavior or stage an explicit migration, release producer support, then the SDK, then the CLI. Validate real published artifact digests. A feature flag, expand/migrate/contract or another compatibility strategy must be stated, not inferred from a successful branch build.

There is no atomic multi-repository merge, no automatic global rollback and no assumption that three green PRs describe one tested vector. If one merge succeeds and another fails, retain the intermediate state and follow the admitted continuation, compensation or forward-fix path. Reverts themselves are new authorized effects, may conflict and cannot erase external consequences.

## Acceptance and integration

Preserve one application decision path for CLI/RPC/embedding. Add focused domain/kernel/protocol modules and adapter seams; avoid one workflow.rs containing parsing, hashing, scheduling, scripts and UI. No crate per tiny record. Runtime/model bridges map every durable boundary and effect result to the actual reducer action.

Qualify races between two runs on one worktree; competing upstream changes; hook descendants; process crashes before/after dispatch; successful remote writes with lost replies; stale review results; failed merge queue checks; partial rollout; restored backups; withdrawn grants; and cancellation during unknown effects. No hardware durability or remote-fencing claim beyond the exercised profile.

Sources reviewed: https://worktrunk.dev/switch/ and https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue. Current upstream behavior does not replace pinned adapter qualification.

Effect transport/completion and domain result are independent. RunCheck may be Confirmed with TestResult.Fail: the check ran and its observed result failed the gate. Effect.Failed denotes the qualified no-effect/rejection outcome, not every nonzero process exit. Preserve domain outcomes such as Fail, Partial, Skipped and Inconclusive inside confirmed observations.

Historical reconstruction validates past transitions against their retained authority/policy/observation context. A later revocation does not make a valid historical event disappear or unreconstructable. New continuation, reuse and dispatch use current applicable grants/corrections in addition to the retained history.

[Plan 27](27-run-watch-and-session-handoff.md) specifies the host’s idempotent intake, task binding, viewer attachment and writer takeover boundary for this procedure.
