# Existing skills as pinned packages and checked procedures

Keep Markdown, Python, shell and assets. No blanket rewrite to Rust and no automatic claim that prose or scripts are verified. The Rust core owns resolution, admission and transition checks; the agent host owns delivering guidance and proposing judgments. Skill import is a candidate operation, not installation plus authority.

## Package and loader contract

An import inventories SKILL.md or an explicitly declared legacy entry, references, scripts, templates, runtime requirements and external skill links. It emits a reviewed `writfold.toml` sidecar and a resolved lock. The sidecar is a Writfold extension, not a claim that Agent Skills or AIP already standardize this schema. Preserve original authoring bytes and locator provenance. Export to host-specific skill formats is a projection with a compatibility report.

Typed edges: `ReadReference`, `RequireContext`, `SelectRoute`, `InvokeProcedure`, `ExecuteAdapter`, and `ProduceArtifact`. A free-text link is navigation until an explicit contract identifies stronger meaning. A source document cannot grant capabilities or add inference rules by containing imperative text. Admission policies and existing user authority govern trusted instruction use.

Discovery supplies small metadata; activation pins one admitted SkillRevision; resources load progressively. Resolve and retain the package closure without injecting every file into context. Required context binds exact resource digests and scope. Record resources offered/delivered and host acknowledgement where supported. These observations do not prove model understanding. Re-supply required context after compaction/resumption where the host cannot establish availability. Do not infer that unloading a document erases model memory.

External references resolve through SkillId and DependencyLockDigest, not an ambient path or newest registry entry. New dependencies follow the existing admission policy; a standing grant can admit an already qualified selection without a new human prompt. Missing/ambiguous identities block that dependency. Same-name packages require explicit namespaced resolution.

## Procedure IR v1

Closed node forms: `Read`, `Select`, `Judge`, `Call`, `Effect`, `Check`, `Join`, and `End`. Each node has a NodeId, input/output type references, required context, applicable scope, capability ceiling, gate floor, dependencies, failure/cancel routes and budgets. A Judge emits a structured proposal with supporting references; correctness is not implied by schema validity. Pure deterministic selection can be verified; open-ended design review remains an evidence-bearing judgment.

The static workflow is finite. Explicit bounded retry/iteration constructs carry a checked budget. Reject arbitrary call recursion and unbounded dynamic code loading in v1. Document reference cycles are allowed through a visited set keyed by resource revision. Do not confuse DAG-shaped call composition with the looping operational lifecycle (retry, review, replan).

A SkillActivation binds RunId, ActivationId, parent invocation, SkillRevision, ProcedureRevision, inputs, repo/context scope, run-binding revision and delegation. Calling another skill can use the same agent session. Optional agent delegation is a separate execution mechanism and receives the same bounded contract.

## Calls, routes and return integrity

At Call: validate the resolved callee export and ABI; establish preconditions; check input bindings; retain required closure; compute child capability as current run grant intersected with parent delegation and admitted callee ceiling. The parent's required gate floor cannot be weakened by a callee. Record the call before starting its effects. Nested calls have unique invocation identities and budget accounting.

At Return: match caller, child invocation, procedure/input revision and expected state. Check required result evidence and postconditions. Outcomes are Success, Failed, Blocked, Cancelled, or UnknownEffect; a generic Boolean `done` cannot replace these. An old child's late completion cannot return into a newer activation. Parallel joins require exactly the declared child results from the same run binding; missing/skipped children keep the join incomplete.

Route selection records the admissible choices, observed inputs and selected branch. Rules such as language=Rust can be deterministic. Ambiguous architectural choices remain explicit judgments. Required branch prerequisites must be checked independently. Changing a route after effects may require replan/rebinding and compensation; never erase the prior branch's effects.

Instruction scope and precedence are explicit: applicable user authority and realm/repository policies remain governing constraints; a skill provides admitted guidance within that scope. Conflicts produce a typed diagnostic and resolution record. Last-loaded text does not win by accident. Repository content and remote comments cannot modify the policy floor through prompt injection.

## Script execution profile

Each executable entry binds script bytes, interpreter, transitive modules/sourced files, dependency locks, executable tool packages, argv schema, working-directory/mount contract, relevant environment, permitted filesystem/network/process effects, resource limits and output schema. A zero exit code is only an observation; independently check the declared postcondition.

Execute through separately admitted Effect Adapter Protocol 1. Read-only Scribe Protocol 1 remains unchanged. Scribes cannot sign, publish, modify source systems, send communications or run arbitrary commands. Reuse supervisor/process mechanics, not Scribe permissions or method names.

The restricted Linux execution profile excludes ambient credentials and mutation paths outside the grant. Descendants share supervision and limits. Scripts requiring unrestricted local execution use an explicit trusted-local profile with weaker guarantees. Merely declaring `effects: read` is not sandbox enforcement. Qualification must demonstrate actual filesystem/network/credential containment and behavior on child escape/cancel races. External provider conditional writes and idempotency remain adapter-specific.

Pinning prevents silent replacement, not nondeterminism. Time, randomness, network state and external observations remain typed inputs or recorded effects. Do not promise reproducible output from arbitrary Python/shell. Package relocation preserves content identity but may invalidate a path-sensitive environment profile.

## Evolution and compatibility

Import advisory skills first, then explicit dependencies/routes, then effect contracts, then qualified procedures. Label each export's assurance: advisory, contract-checked, or verified-components, with its exact evidence. No global `verified skill` badge inferred from a valid manifest.

AIP is an optional interchange investigation: pin its specification/tool versions, define a lossless supported subset, reject unrepresentable semantics and test round trips. No dependency on a moving upstream schema, no automatic execution of imported scripts, and no substitution of AIP runtime claims for Writfold proof obligations.

Learning proposes patches to the correct object: metadata trigger, router, guidance, script, gate or dependency binding. Re-evaluate relevant held-out cases, regenerate the sidecar/lock where needed, and admit a new revision. Urgent withdrawal can block future effects of a pinned revision; resumption requires an explicit migration/replan, not an invisible hot swap.

Required campaigns: nested cross-skill loading, missing resources, Markdown cycles, rejected call recursion, route mismatch, stale child result, budget exhaustion, dependency mutation, changed interpreter, insufficient capability, policy-floor bypass, loader failure after compaction, and script output that lies about its postcondition.

Sources: https://agentskills.io/specification and https://github.com/zach-blumenfeld/aip (reviewed 3 November 2026). The proposed Writfold contracts above go beyond those formats and remain unimplemented.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.

Revision 7 uses the [session-driver package candidate](../examples/run-watch/session-driver/SKILL.md) for activation and handoff qualification. Its local reference, compiled lock, host acknowledgement and candidate-specific certificate belong to the package admission closure.
