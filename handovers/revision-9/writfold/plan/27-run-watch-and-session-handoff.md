# Run Watch and the session handoff

Tarek selected the five screens as the human design baseline on 4 November 2026. Gate inspector is the proposed default. The Pipeline, Citation and Effects tabs expose dependencies, supporting spans and the history of external operations. The [approved screens](../examples/run-watch/approved-screens.pdf) are one synthetic replay. Their approval settles the visual target for this handover. Product admission, command availability and a driver's authority each have their own gates. The [citation queue](../PRIORITIES.md) still leads.

## From chat to a second terminal

1. Tarek asks for a tracked, multi-repo change with a concrete finish line, such as “prepare reviewed patches for API, SDK and CLI”. A configured author-change procedure may default to tracking for eligible change tasks. A question, exploratory chat or request for an existing ID performs a lookup only. If the scope or write grant is missing, explain what is blocked. Do not invent activity before tracking began.
2. The host resolves the one authorized WSL realm and recovers the task's durable binding. A run per logical change can span several chats. Two changes in one chat can have independent runs. A chat/session ID, branch name or repo directory is insufficient as the run key.
3. If bound, read the existing run. If authorized to start a new one, persist an opaque request key for this task **before** submission. The coordinator conditionally records the request-key mapping and first ledger event together. It returns a typed RunRef after durable commit. A new run is Proposed until preparation and activation are independently recorded.
4. The agent sends a short receipt immediately. Example from the fixture: “Tracking the pagination change across API, SDK and CLI. Run `demo/RUN-017`. Open it in another WSL terminal with `writfold run watch demo/RUN-017`. I’ll stop at reviewed patches.” It continues the authorized work in its own session. In this example, opening PRs requires a later explicit change of finish line and grant. The approved mock replay instead illustrates a task whose scope already includes opening a PR.
5. The human pastes the command from any directory in the same configured WSL environment. The TUI opens Gates at the latest available consistent cut, highlighting a blocker or an unresolved effect if one exists. If neither exists, select the current step or final outcome. Tabs and evidence inspection are read-only. Closing the terminal detaches that viewer; the agent and run continue under their own lifecycle. Opening a watch does not start a worker. If the only worker stops, its work pauses until a qualified driver takes over.
6. On “where are we?” report the same ref and copyable command plus state, exact cut, one blocker or achieved boundary, and one next step. For explicit past evidence include a pinned snapshot command. On “continue this run”, restore the bindings as below before taking any action. At closure report what actually completed, any outstanding effects, and the command to inspect the record.

## CLI contract for the implementing agent

Proposed v1 spelling. Examples assume an installed binary and provisioned realm on the WSL host. Use the same typed query/command service from CLI and JSON-RPC. Scripts should use the structured result. The handoff has no dependency on an interactive chat continuing to run.

| Operation | Surface | Meaning |
| --- | --- | --- |
| Start or recover an acknowledged start | `writfold run start --request intake.json --json` | Mutating intake for a configured driver. Return a committed RunRef, one start event and whether the existing mapping was reused. See [sample intake](../examples/run-watch/start.json) and [acknowledgement](../examples/run-watch/start-result.json). |
| Watch | `writfold run watch demo/RUN-017` | Human-only live reader. Follow coherent cuts, default to Gates. Press 2 for Pipeline, Enter for evidence, Esc to return, q to detach. Opening two viewers has no effect on ownership. |
| Snapshot | `writfold run show demo/RUN-017` | Read once, printable on a pipe or without a TTY. `--json` uses stdout for the versioned record, stderr for diagnostics. Successful observation of a blocked run still exits zero. |
| Preserve an exact observation | `writfold run show demo/RUN-017 --at 34 --json` | Read the retained prefix through event 34. If unavailable, report that fact. Do not silently substitute latest. Show the pinned release, view cut and sampled ledger head separately. |
| Find a lost reference | `writfold run list --realm demo` | List only readable runs in the named realm, with ref, objective, boundary and last-known event. Multiple plausible matches require selection; never attach a writer to the most recent result automatically. |

Use noun then verb. A public RunRef has the spelling `<realm-id>/<run-id>`, where both parts are validated, opaque, shell-safe tokens. `demo/RUN-017` is a synthetic example. A bare ID may be resolved only with a unique explicitly configured realm. A watch command copied across worktrees must still resolve the same logical run. A RunRef carries no read or write grant. Resolve in the selected local WSL store, authenticate each read and filter all output by visibility. For an unmapped realm, report how to select the correct configured environment. If this chat cannot reach the coordinated host, say so rather than producing an ID. Non-TTY watch tells the caller to use show and exits. A read error, an unavailable cut or a missing run returns nonzero. Unknown and unreadable identifiers share a safe public diagnostic.

## What moves on the screen

Create the run before Worktrunk checkout. The driver passes its bound RunRef to the admitted author-change procedure. That procedure asks the controlled adapters to prepare the worktree, run repository checks and open an authorized PR. Each operation records its own typed event or receipt against the actual repo revision. The projector reads those committed events and advances the view watermark. The terminal can then show the result. Work performed in an ordinary shell outside this route is an unobserved action until qualified evidence is imported. A chat update such as “tests passed” is an agent observation; the gate changes only when validated test evidence for the bound inputs is recorded. User feedback can create a learning candidate linked to the run without changing the current procedure.

## Write and recovery boundary

Key uniqueness is scoped to authenticated realm, initiating principal and request key. Payload equality compares a canonical digest of the immutable start request, including objective, repo identities, procedure, authority reference and completion boundary. One conditional transaction or replay-equivalent single ledger append binds key, digest, RunId and initial event. Same key and digest returns the original start; different bytes with different semantics under the same key conflict. A timeout, crash or competing process repeats the **identical** request with the persisted key. Losing its key requires explicit binding recovery. Creating a new key while the first outcome is unknown can fork the task. Never infer identity from equal titles. If atomic storage or durable host bindings are unavailable, block start rather than emulating either with an agent guess.

The host stores the returned RunRef and key with the logical task. After compaction or a new chat, read the exact RunRef; reload the pinned driver/procedure and required context. Inspect the authoritative head, binding revision, procedure, repo vector, completion boundary, grant, owner generation and unresolved effects. Only an authorized coordinator transfer with proven quiescence reassigns a live writer. A reader can inspect a run while another session drives it. A lost forge response leaves the same EffectId unknown until adapter reconciliation. Claim persistence precedes I/O. Do not redispatch just because an agent returned to the chat.

A session host may use a launch hook to recover a binding and a close hook to propose a learning observation. Hooks use the same contracts as an explicit invocation. Closing a chat does not close the run or admit a lesson. Worktrunk, document refresh, checks and PR adapters remain under their own procedure, capabilities and lineage.

## Read model and implementation order

The ledger selects legal transitions. The activity graph is a projection with a watermark. Both watch and show render one immutable RunView at a time. Anchor it to the logical run, read scope, procedure and repo pins, corpus release, ledger prefix and projection epoch/cut. Label a sampled head as last observed. A projection behind the ledger can show “2 behind”. On disconnection retain the last verified snapshot, label it disconnected, and reacquire a consistent cut. On sequence gaps, deletion or epoch mismatch, reread and replace the snapshot. A history browser never pairs old gates with newer event details. Logs, agent prose and mock states cannot advance progress.

| Slice | Owner and exit |
| --- | --- |
| 1. Reference and discovery | P14/WF25. Implement exact RunRef parsing, atomic receipt and idempotent intake recovery, scoped discovery and no-state-change reads. Start-conflict and response-loss schedules have executable replay cases. |
| 2. Agent driver | P15/WF26 and P16/WF27. Import the narrow [driver candidate](../examples/run-watch/session-driver/SKILL.md) and lock its references. Check procedure composition and driver takeover under the qualified authority model. |
| 3. Real workflow | P17/WF28. The three-repo fixture exercises read-back after new chat, target change, stale evidence and an unknown forge effect. All mutations use the existing guarded service. |
| 4. Terminal interface | P19/WF30 and P11. Ship plain show first. Pilot Ratatui only over a prepared view, using the five approved screens. Keep the earlier corpus/ledger read boundaries. WSL Linux only, including the installed release path and the external BuildKit setup. |

Use nominal RunId, RealmId, StartRequestKey, RunRef, event sequence and cut types. Keep command parsing, engine service, ledger adapter, snapshot builder and terminal rendering individually testable. Thin CLI/RPC adapters call one service. Retain the 400-line module and 80-line function review thresholds. Do not add UI-only authority checks as a substitute for the core.

## Verification and acceptance

The proof owner maps new start uniqueness, response loss, reader noninterference and transfer rules into the existing Lean engineering obligations. If an obligation is insufficient, propose and prove the delta before enabling the path. Use Verus for the Rust checked predicate/refinement boundary, with explicit trusted persistence/IO assumptions. Use TLA+ for start/retry/crash, readers, generation change and lost forge replies; model-check finite fault schedules and discharge the applicable invariants with TLAPS. Record assumptions, theorem symbols and artifact hashes per plan/25. An approved screen or successful fixture cannot certify a particular skill/procedure candidate. Its Lean-checked instance binds exact bytes, witness, predecessor and intended use; enable neither driver nor policy before all applicable receipts exist.

Qualification must demonstrate these cases from ordinary entry points.

| Case | Expected observation |
| --- | --- |
| First start, duplicate response, concurrent identical starts | One durable run, one start event, same returned ref. Different intent under the same key reports conflict. |
| Ask for ID twice, then ask from a new session | Zero writes for repeated lookup. A known ref gives the same command. Ambiguous candidates surface a choice. Untracked activity is never retrospectively labelled as observed. |
| Launch from a sibling worktree, outside a Git repo, in another realm, without a TTY | Authorized reader opens the same run regardless of current directory. Wrong realm fails safely. Piped show yields text/JSON; piped watch gives the snapshot command. |
| Two watchers attach and quit during execution | Unchanged writer generation, event count and grant in a quiescent fixture. Quitting one view does not cancel the run. |
| Reconnect with a gap; concurrent repo rebase | No mixed cuts. Label lag, keep the old receipt historical, require revision-bound evidence before showing pass. |
| Two owners claim a run; PR reply lost | Only a valid fenced owner can effect a change. Unknown retains reservation and EffectId. An independent lookup can confirm the PR. The observer issues no retry. |
| Human pilot on WSL | Compare show with watch on the same replay. Ask the reviewer to identify blocker, repo revision, next evidence and PR outcome. Archive answers, timings, terminal dimensions, missing coverage and an 80-column fallback. Admit production UI only through normal proof and review gates. |
