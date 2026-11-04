# Learning as a reviewed, evidence-bearing change

The session hook becomes an observation/reconciliation trigger. It may run once per session, but observations also arrive from test failures, PR comments, agent decisions, rollout incidents, changed assumptions and Tarek's feedback. Persist observable inputs/results and concise explicit rationale; do not depend on hidden model chain-of-thought or a complete conversation remaining in context.

## Records and independent dimensions

Observation is a retained event with source, exact revision, acquisition outcome and time. It does not become a usable claim merely because it appears in the activity graph. Candidate kinds are KnowledgeCorrection, ProcedurePatch, GuidancePatch, ToolRepair, ScopedPreference and PolicyChange. Keep factual support, applicability, novelty, evaluation, approval and publication separate.

Learning lifecycle: Observed -> Proposed -> Scoped -> Evaluating -> Reviewed -> Admitted / Rejected / Deferred / Superseded. The record retains original observations, hypotheses, supporting and contrary evidence, claimed applicability, counterexamples, proposed exact delta and intended target predecessor. A refuted lesson stays historically inspectable without being selected as useful knowledge.

A feedback source has an explicit role. A reviewer comment can be a defect report, question or preference; it is not automatically organization policy. A clear user instruction is scoped to its stated intent and authority. Do not infer global preference from one task-specific correction. A timeout is not proof of a code defect. A successful deployment alone does not establish causal superiority of a procedure.

## Evaluation contract

Before evaluation freeze: candidate bytes, baseline procedure/tool revision, task/domain scope, evaluator and rubric, datasets/fixtures, expected cost budget, held-out partition, adverse cases, acceptance thresholds and permitted exceptions. Log all trials and exclusions. The candidate must not rewrite its own rubric or select only favorable results after observing them. Evaluation protocol changes create a new evaluation revision.

Measure task correctness, missed/introduced defects, false blocking, recovery behavior, completion quality, human corrections, tokens, latency and maintenance cost as relevant. Do not optimize for merely receiving fewer review comments or persuading a reviewer. A smaller token count cannot excuse lost mandatory evidence. Experimental usefulness is a scoped empirical result, not a theorem of universal improvement.

Use the original incident for reproduction and separate cases for generalization. Compare equivalent scripts/tools where testing the effect of procedure structure; otherwise report the confound. Negative cases must exercise applicability boundaries. Evaluation can conclude insufficient evidence and leave a useful hypothesis provisional. Tarek need not approve every observation; existing policy may admit bounded factual corrections while reserving broader procedure/policy changes for review.

## Example

An old SDK rejects an added API response field. Retain decoder/source revisions, request/response bytes and a reproducing test. Repair the current change immediately through its design/implementation loop. Separately propose a scoped rule to inspect supported consumers and test old-consumer/new-producer combinations for response-schema extensions. Evaluate it on unrelated schema changes and cases where the extra check is unnecessary. An accepted improvement becomes a new author-change ProcedureRevision with explicit applicability.

## Admission and non-amplification

Learning admission binds exact candidate/evidence/evaluation closure, current target predecessor, applicable scope and authorization. Reuse existing knowledge witness/context checks for any new factual claims. Procedure changes additionally pass package/type/effect checks, gate-floor preservation, compatibility and evaluation policy. Metadata `approved=true`, an LLM confidence score or a passing test cannot bypass these checks.

The improvement may strengthen a procedure or revise its heuristic guidance. It cannot remove the realm/kernel gate floor, increase effective privileges, reinterpret previous receipts or modify the verifier/evaluator to authorize itself. Such policy/kernel changes are separate explicitly authorized engineering changes with their own proofs and evaluation. This boundary prevents infinite self-approval: the governing admission mechanism is outside the candidate's delegated authority.

Retain evidence in corpus closure before the completed Git release CAS. Learning status acknowledgement follows publication and is reconciled by exact operation ID on interruption. Duplicate same-ID/same-input promotion reports its original result; same-ID/different-input conflicts. Competing promotions against one predecessor require a new reviewed operation rather than silently rebasing old approval.

Existing runs keep their admitted procedure/read pins. Withdrawal or correction is current policy input for future effects: block affected actions and require explicit migration/replan where necessary. Do not rewrite old runs or turn retrieval into a new observation time. Independent supporting witnesses retain their existing semantics.

## Views, retention and privacy

Activity queries expose candidate state, evidence completeness and evaluation limitations. Obsidian presents observations, proposals and admitted knowledge with distinct status labels and exact read cuts. Generated summaries are either deterministic presentations or explicitly non-authoritative editorial material. Export filtering applies to filenames, links, counts and attachments as in plan/13.

Raw session logs are not mandatory learning content. Capture the minimum relevant evidence; preserve original acquisition/source identities, access controls and redaction disposition. A redacted copy has a distinct object identity. If required evidence cannot be retained, mark the candidate incomplete rather than inventing a complete closure. Never put secrets in fingerprints or public assurance pages.

Admission evidence outlives the activity projection. Pruning a dashboard/run index cannot erase a published lesson's justification. Retention of the underlying observation object is governed by all roots, including publication and active evaluations. First implementation disables destructive GC and proves the bridge before enabling it.

## Gates

Test wrong-scope promotion, insufficient/contrary evidence, post-hoc threshold changes, training/evaluation leakage, repeated-incident deduplication, fake authority metadata, policy-floor weakening, lost publication acknowledgement, activity deletion, correction after promotion, and stale in-flight activations. Learning usefulness and formal admission safety have separate receipts and status.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.
