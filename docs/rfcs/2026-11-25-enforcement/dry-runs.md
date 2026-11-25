# Ten enforcement design dry runs

Executed with the standard-library Python reference model [dry_run.py](dry_run.py).
**10 scenarios, 43 assertions passed.**
Machine-readable events and checks: [dry-run-results.json](dry-run-results.json).

This exercises a proposed protocol under explicit assumptions. It does not run Writfold,
a live model, a real harness, a proof kernel, or a protected production executor. Passing
means the modeled decisions are internally consistent for these traces; it does not
establish that the current product implements them or that a host cannot bypass them.

| # | Scenario | Observed modeled outcome | Design consequence |
|---|---|---|---|
| 01 | Complete research and delivery | Allowed | Preparation, observation, checking and sending have separate owners. |
| 02 | Startup hook never runs | Blocked, then repaired | The protected action path must check binding independently of startup. |
| 03 | Duplicate start and wrong task | Replay reused; invalid bindings blocked | Session identity is not task identity; bindings and bootstrap must be explicit. |
| 04 | Skill omitted or only prepared | Blocked before submission | A prepared packet is not evidence that a model call occurred. |
| 05 | Compaction before dispatch | Stale context blocked; fresh context admitted | Context changes require fresh preparation and use-time checks. |
| 06 | Incomplete or failed KB search | Blocked, then repaired | The enumerator owns completeness observations; missing input is not empty success. |
| 07 | Eight of ten comparisons | Blocked until two reports added | Report recording, coverage and graph advancement are distinct. |
| 08 | A newly related sixth document | Old completeness refused; eligible units reused | Scope membership and per-unit evidence need different dependency sets. |
| 09 | Model/harness switch and CLI-only use | Supported work admitted; strict unsupported work blocked | Capabilities belong to operations; changing host need not discard the KB. |
| 10 | Forged evidence, altered draft and bypass | Protected path refuses; raw bypass remains possible | Only the effect owner can enforce; a receiver cannot undo earlier outside effects. |

## What the dry runs clarified

1. Bootstrap must be ordinary adapter/supervisor code. The model is not responsible for remembering to create a run.
2. Startup automation and enforcement are different. An independent protected operation path must reject a missing binding even when startup never ran.
3. Run binding is per task as well as session/workspace. Replayed startup is idempotent; new tasks, closed runs and mismatched bindings cannot borrow an ambient current-run pointer.
4. Context prepared, request submitted, result recorded and graph advanced are distinct transitions. No receipt may skip those distinctions.
5. The run/scope manifest owns collection completeness; individual reports bind their actual unit inputs. A new document invalidates completeness but can leave earlier pair reports reusable under policy.
6. Missing harness capabilities block the affected strict obligation. Tool-owned evidence and independent receiver checks still support useful portable workflows.
7. An adapter translates and invokes checks. The host or protected receiver owning the effect must actually obey the decision. Scenario 10 intentionally demonstrates that an unmediated external action can happen.

## Responsibilities made explicit

| Component or role | Must do | Must not be assumed to do |
|---|---|---|
| Operator and admitted policy | Select the workflow, baseline requirements, allowed issuers and deployment boundary | Let model classification silently remove baseline obligations |
| Harness or task-entry supervisor | Invoke deterministic bootstrap before governed work; hold calls until admission; own task/session lifecycle | Infer that every user message is a new task, or assume a missing startup callback enforced anything |
| Harness adapter | Translate host events, create/attach the run, bind the task/session, prepare context and submit faithful observations | Decide its own policy, claim stronger evidence than the host exposes, or block an effect it does not control |
| Writfold pure kernels | Judge bindings, obligations, applicability, coverage and graph transitions | Observe a model call, run a tool or physically prevent an external action |
| Writfold CLI and record IO | Persist runs/evidence and publish decisions against the checked state | Treat a worker-editable current-run pointer or issuer string as protected authority |
| Model/agent | Propose plans/actions and produce attributed assessments and drafts | Own mandatory bootstrap, mint trusted receipts, waive gates or mark checked progress by assertion |
| Tools and checkers | Produce version-bound results and observations for operations they actually perform | Prove global semantic relevance or model understanding from structural checks |
| Effect owner or protected receiver | Apply admission at the actual operation and use the checked immutable inputs | Undo earlier effects through uncontrolled paths or infer compliance from the mere existence of a run |

Several roles can live in the same harness process. This is not a proposal for eight
services or a Writfold daemon. In the Python precursor, ordinary wrapper functions can
own bootstrap and dispatch; in another harness, an extension may supply the callbacks.
The protected effect owner must still honor denial and cover the claimed paths.

## One complete interaction sequence

1. The user submits or explicitly resumes a task through the configured entry point.
2. The supervisor calls the adapter; the adapter asks Writfold to create or attach an admissible run and retains the exact task binding. Replayed bootstrap reuses it.
3. Writfold returns current obligations. The supervisor invokes mandatory preparation tools, whose observations are recorded and checked.
4. The adapter assembles required skill/input content. Writfold checks the prepared invocation; the model-call owner dispatches only when allowed.
5. The harness records the actual submitted request and associates returned output/tool proposals with that invocation.
6. The effect owner checks the binding and relevant obligations before each governed tool action, executes allowed work, and reports what happened.
7. The model produces results; tools/checkers validate and record them. Writfold separately checks whether graph advancement is permitted.
8. The delivery checker validates exact bytes; the actual sender checks current authority and publishes only those bytes. Writfold retains the outcome and unresolved work.

Task entry can be a native extension callback or a controlled SDK wrapper. A launch
wrapper only establishes startup unless it also controls later operations. Session start
may not yet contain a task: the binding must be established at an explicit task boundary
or before the first governed action. Compaction is not automatically a new task.

## Reproduce

```sh
python docs/rfcs/2026-11-25-enforcement/dry_run.py --output-dir /tmp/writfold-design-dry-run
```

Outputs are deterministic. All documents, calls, effects and issuer identities are synthetic.
The reference model is a review aid, not a second implementation to keep compatible with
the product. The accepted Rust decisions still require extraction proofs and real adapter
conformance. Dispatch crashes, races, distributed permits, signature validation, natural-
language correctness and actual plugin availability remain outside this exercise.
