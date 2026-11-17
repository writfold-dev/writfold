# v0.6.0-dev.6 and attached stores: specification

> **Status:** planned; nothing here is released. Part A (the feedback release) is drafted in the working tree of branch `session-runs` and on branch `repin`, uncommitted or unmerged, and has not passed the proofs gate in full or the quality and unit gates; [section 7](#7-state-of-the-work-and-order-of-implementation) says exactly what exists. Part B (attached stores, [section 8](#8-part-b-attached-stores-for-disposable-evidence)) is a design only. Written against `main` at `4f73917` (v0.6.0-dev.5). The authority is [mission.md](mission.md). Where sections 1 to 6 say "built", "pinned by a test" or "now", read "drafted and locally tested" until the release is gated.

This release answers a review by someone moving a working agent flow onto the `writfold` binary: hooks of their own, a registry of checks, and several agent sessions running at once on one machine against one KB clone. Of the earlier round's asks, most had shipped. One thing still blocked the move, and it is general: **every session on a KB clone shared one current run**. The rest of the review is smaller requests and gaps the flow implies. Each need below is stated for any user; nothing in it is specific to one setup.

Each description of today's behaviour was checked against the code at `4f73917`; what was found is in [section 6](#6-what-was-checked).

## 1. Triage

| # | Need | Verdict | Reason |
|---|---|---|---|
| 1 | Parallel agent sessions on one KB clone, each on its own task, without sharing a current run | **Adopt: a current run per session and per work tree, with the shared pointer as the fallback** ([2.1](#21-a-current-run-per-session-and-per-work-tree-item-1)) | The one blocker. With one pointer per clone, a second session's new run moves the first session's pointer, and the first session's next commit is judged against gates it never owed, or passes on gates the other session met. `--run` on verbs does not help: a hook cannot be given one |
| 2 | Several lookups in one process, for hooks | **Adopt: `find --each`** ([2.2](#22-find---each-item-2)); reverses the "decline" of [dev10-spec 4.6](dev10-spec.md#46-lookup-layers-not-built-item-7) | That decision rested on one `find` taking about 10 ms. On a KB of 3,000 records it measures 48 to 92 ms with process start, and a hook that resolves three names before each search pays it three times at every tool call |
| 3 | A run left `policy-moved` by a gate edited mid-run has no way forward but a new run | **Adopt: `plan repin`**, as specified in [policy-spec 2.5](policy-spec.md#25-policy-pins-and-drift-items-11-12) | Planned since v0.6.0-dev.3; tuning gates on live runs makes it frequent |
| 4 | Gates in the KB root that apply to some repositories only | **Already does through plans: confirm, test and document. Defer a `repos` key on a gate** ([2.3](#23-gates-for-some-repositories-item-4), [5](#5-deferred-with-reasons)) | A run's plan narrows what the run owes to the gates its steps name. That covers a KB that holds gates for several kinds of repository. A selector on the gate itself raises questions a plan does not |
| 5 | Upgrading on purpose: what changed in behaviour between releases | **Adopt: one page, kept with each release** ([2.4](#24-behaviour-changes-by-release-item-5)) | Five pre-releases in two days carried behaviour changes spread over five sets of notes |
| 6 | A short path for small changes that still leaves a record | **Adopt as a documented recipe, not a built-in** ([2.5](#25-recipes-items-6-to-10)) | An attested gate and a one-step plan template already give the record and the narrowing. A rule-level waiver stays declined ([dev10-spec 4.2](dev10-spec.md#42-a-rule-level-waiver-guard-waive-item-3)) |
| 7 | Rolling guards out without false refusals | **Already does (`mode = "audit"`, `guard log`); adopt the rollout as a recipe** | The mechanism exists; what was missing is saying how to use it |
| 8 | Converting an existing registry of checks | **Adopt as a recipe** | Mechanical, checklist and attested gates, plan templates per kind of change and `kb lint` are the pieces; the mapping was not written down |
| 9 | Sources reachable only through an agent's tool server | **Adopt as a recipe** (a lifetime, an observation, or a checker over any command-line route) | A checker is a plain process and stays one |
| 10 | Look in the KB before searching code | **Adopt as a hook recipe over `find --each`; not a plugin hook** | What counts as a name worth looking up is the user's to tune, and the hook reads harness events |
| 11 | Reviewers, subagents and second sessions working in one run | **Adopt: `resume <run> --here`**, with item 1 | The run is the spine of a flow. With a pointer per session, a second session needs a way to take an existing run without writing a tracked file |
| 12 | Citations rendered as links in outward text | **Defer** ([5](#5-deferred-with-reasons)) | Needs a design for where link templates live and what an unmapped repository renders as |
| 13 | Recorded verdicts, or a checker override, for the built-in `code` kind | **Defer** ([5](#5-deferred-with-reasons)) | Changes which verdict wins for a kind checked from local clones: decision logic that needs its theorems first |
| 14 | Batch `observe --from` and `record --from`, for importing history | **Defer** ([5](#5-deferred-with-reasons)) | An imported look must never read as one made in the run; the marking is undecided |

Not asked for and not built: signing or access control, semantic search, a built-in hard stop.

The cut line: items 1, 2, 3 and 11 are code; 4 to 10 are tests and documentation. Item 1 is the release; everything else was taken only as far as it could land proved and tested.

## 2. Adopted items

### 2.1 A current run per session and per work tree (item 1)

**Today.** `<KB git dir>/writfold/current-run` is one file. `checkpoint` writes it; every hook and every verb without `--run` reads it (`runs::current` and `runs::current_of`, fifteen call sites). `guard status` takes no `--run`, and a `PreToolUse` hook cannot be given one. So the pointer is shared by every session on the clone.

**The fault.** Two sessions on one machine, in different repositories, each on its own task. Session B's `checkpoint --new-run` moves the pointer. Session A's next `git commit` is judged against B's run: refused for gates A never owed, or passed on gates B met.

**Design.** A *seat* is one agent session working in one work tree. A KB clone keeps three kinds of local pointer, and a seat reads the most specific one that is set:

| Order | Pointer | File under `<KB git dir>/writfold/` | The seat has it when |
|---|---|---|---|
| 1 | the session's | `current-run.d/session-<session id>` | the session is known: a hook event's `session_id`, or `$WRITFOLD_SESSION` for a verb |
| 2 | the work tree's | `current-run.d/tree-<16 hex of the SHA-256 of its canonical top>` | the directory (a hook's `cwd`; for a guarded command, the directory it acts in; for a verb, the working directory) is inside a Git work tree that is not the KB's own |
| 3 | the shared one | `current-run` | always |

The decisions, and why each was taken:

- **Session first, then work tree, then shared.** A session is the unit of work an agent flow has; the work tree is what a plain shell and a fresh session have. Session first handles two sessions in one work tree and a session that works across repositories. Work tree second needs no setup at all (hooks and verbs both know their directory), and it alone already removes the reported fault. Shared last keeps every existing setup working unchanged.
- **Both narrower levels, not one.** A session pointer alone fails where the session is unknown: a person's shell, a harness that does not pass a session ID, a new session resuming yesterday's work in the same worktree. A work-tree pointer alone fails for two sessions in one work tree and misjudges a session that commits in a second repository.
- **The most specific set pointer decides whatever its run's state.** A session whose run was closed is told so. Falling through to a less specific pointer would judge it against a run that may be another session's, which is the fault again. So there is no "stale pointer" handling: a pointer to a closed or missing run is a fact that is reported (`run <id> is closed`, `… is not a run in the KB`), and a new run or `resume <run> --here` replaces it.
- **Making a run current writes every pointer the seat has, and the shared one always.** The shared pointer stays "the run last made current on this clone", which is what a shell outside every work tree, and `writfold watch` in the KB, should follow. Nothing is cleared on close, for the reason above.
- **The KB's own work tree has no work-tree pointer.** A run is not usually worked in the KB. If it had one, `watch` running in the KB would stick to whichever run was last started from inside the KB instead of following the clone. When the KB lives inside the work repository, the work-tree level and the shared level are then the same thing.
- **Pointers live in the KB's Git directory, not in each work repository's.** One KB clone then holds all of them (a reader can list them, deleting one directory resets them), nothing is written into repositories the user may not own, and two KBs used with one repository do not collide.
- **How the session reaches verbs.** Claude Code gives the session ID to hooks, not to the Bash tool. The `SessionStart` hook appends `export WRITFOLD_SESSION=<id>` to the file Claude Code names in `CLAUDE_ENV_FILE`, which it runs before each Bash command of the session. `WRITFOLD_SESSION` already existed for `propose`. A value that is not a session ID (the draft rule: 1 to 128 of `[A-Za-z0-9._-]`, no leading dot) is ignored, in a hook event and in the variable alike.
- **Attaching.** `writfold kb resume <run> --here` makes an open run current for the seat (local pointers only) and then answers as `resume`. It refuses a closed run. A subagent whose hook events carry the parent session's `session_id` reads the parent's run with no step.
- **Crash points.** Each pointer is one file written by one atomic rename, the most specific first. A crash between two leaves the seat itself reading the new run and at worst a less specific pointer on the earlier one. Every combination of pointer states is a valid input of the kernel below, and the theorems cover all of them, so there is no multi-step protocol whose intermediate states need a model; `spec/RunGraph.tla` and `spec/KbWrite.tla` do not model the pointer and are unchanged.

**Theorems** (`proofs/Writfold/Pointer.lean`, over the translation of `writfold_core::pointer`; proved and mutant-calibrated before `writfold-kb` called the kernel):

| Theorem | Statement |
|---|---|
| `is_set_spec` | a pointer is set exactly when it is not `Unset` |
| `level_spec` | the deciding level is `Session` exactly when the session pointer is set; `Tree` exactly when it is not and the work tree's is; `Shared` exactly when neither is and the shared one is; none exactly when none is set |
| `open_spec` | the seat's current run is open exactly when the deciding pointer names an open run |
| `session_decides` | with the session pointer set, `open` is the same for every value of the other two pointers |
| `tree_decides` | with no session pointer and the work tree's set, `open` is the same for every value of the shared pointer |
| `ended_not_skipped` | when the deciding pointer names a closed or missing run, the seat has no open current run, whatever the less specific pointers name |
| `shared_alone` | with only the shared pointer, the answer is that pointer's (the rule of every earlier release) |
| `writes_spec` | a seat writes the session pointer exactly when it has a session, the work tree's exactly when it has a work tree, and the shared one always |
| `writer_reads_its_run` | after a seat makes an open run current, it reads an open current run, for every prior state of the three pointers and every seat shape |
| `other_session_undisturbed` | a seat whose session pointer is set reads the same before and after another seat makes a run current |

Six mutants, each rejected by the theorems named in [aeneas.md](aeneas.md#the-claims-and-their-mutants): work tree before session, a missing run treated as unset, a closed session run skipped for an open work-tree run, any run counted as open at the shared level, the shared pointer not always written, the session pointer never written.

**Tested, not proved** (`crates/writfold-kb/src/seat.rs`, `hook/export.rs`): reading the pointer files; the state of the run a pointer names (the manifest's presence, the sidecar's or the manifest's `state`); which work tree encloses a directory and whether it is the KB's own; the form of a session ID; the `export` line; argument parsing; the notes `guard status` and `resume --here` print. `resume --here` refuses a closed run through the already proved `guard::require::run_current`.

**Tests.** `crates/writfold-kb/tests/seat_runs.rs`: the reported fault end to end (two repositories, two runs, the guard hook judged per repository; deleting the narrower pointers brings the fault back, so the work-tree read is what a test depends on); a session pointer deciding before the work tree's; a closed and a missing run reported, not skipped; `resume --here` and its refusal; no work-tree pointer inside the KB. `crates/writfold/tests/kb_session_runs.rs`: through the binary, the hook's export and two sessions each resuming their own run. Kernel unit tests cover all 64 pointer combinations.

### 2.2 `find --each` (item 2)

`writfold kb find --each Q… [--kind K] [--project P]` answers each argument as its own query, in the order given, in one envelope: a row of kind `query` (`hit` or `none`, `hits`), then the first five rows a lone `find` gives for it, each carrying `query` in JSON; `!more` when there are more; a miss keeps `find`'s suggested names under it, flagged `!suggested`. The trailer counts `queries` and `miss`; any miss exits 2, as a lone `find` that finds nothing does. `find a b c` without the flag stays one phrase.

Five rows per query, because one broad query otherwise fills the page and the later queries are cut (seen on the fixture KB: `harbor api` gives 25 rows and the second query's rows fell off a 1,000-token page). Five is the measure `find --eval` already uses.

No decision is added: each query runs `find`'s tiers in `find`'s order through the proved `find::admit`, with a smaller row limit. Tested (`crates/writfold-kb/tests/find_each.rs`): each group equals the first five rows of the lone answer; a miss is counted, exits 2 and flags its suggestions.

### 2.3 Gates for some repositories (item 4)

A run's plan narrows what the run owes: `plan::owed` gives `gate status`, `gate lines`, `gates.met` and `resume` the gates the plan's steps name, and every defined gate only when the run has no plan. So a KB that holds `py-unit` and `cpp-unit` in its root, and plan templates per kind of change, already asks a Python change for `py-unit` only. This was true and untested for the guard; it is now pinned (`seat_runs.rs`, `a_plan_narrows_the_kb_root_gates_a_run_owes`) and documented ([adopting](../guide/adopting.md#4-gates-for-some-repositories-only)).

### 2.4 Behaviour changes by release (item 5)

[`docs/guide/changes.md`](../guide/changes.md): per release, only what an existing setup can notice (a different answer, exit code, file or default for input that was already valid), newest first, with fixes marked. Seeded from the v0.6.0-dev.1 to dev.5 release notes; each release from this one on adds its section, and the release notes keep a "Behaviour changes" heading that the page copies.

### 2.5 Recipes (items 6 to 10)

[`docs/guide/adopting.md`](../guide/adopting.md), each recipe run against a scratch KB with this release's binary: guards in audit first; several sessions; a short path (`lite` plan template and an attested `lite` gate: the run reads `complete-with-exceptions … 1 attested`); gates per kind of change; converting a registry of checks; sources only an agent's tools reach; a lookup hook over `find --each`; moving history. Its TOML blocks are read by the real parsers in `guide_toml.rs`.

### 2.6 `plan repin` (item 3)

As [policy-spec 2.5](policy-spec.md#25-policy-pins-and-drift-items-11-12) specifies; its theorems and what is tested are recorded there and in [aeneas.md](aeneas.md).

## 3. Build order

1. `writfold_core::pointer`, its unit tests, `start_from`; translate; `Pointer.lean`; audit claims; mutants calibrated; the proofs gate.
2. `writfold-kb`: `seat.rs` (files), `Workspace.seat`, `runs::current_in|current_of|current_at`; every reader moved; `checkpoint` writing through `seat::make_current`; the guard judging each command with the seat of the directory it acts in.
3. The hook: the event's `session_id` as the seat's session; the `CLAUDE_ENV_FILE` export.
4. `resume <run> --here`; `find --each`.
5. Tests; guides (`hooks`, `kb-layout`, `feeding-back`, `gates`, `watch`, `json`, `assurance`, `adopting`, `changes`); the plugin's skill and hook description; this page.
6. `plan repin`, in its own branch, merged after its own proofs.
7. The proofs gate on the merged tree; `scripts/gate quality` and `unit` from main; release.

## 4. Compatibility

- A KB clone with only `current-run` reads exactly as before (`shared_alone`).
- An earlier binary ignores `current-run.d/` and reads `current-run`, which every `checkpoint` still writes.
- A script that sets the current run by writing `current-run` is not read by a seat whose own pointer is set. This is the one behaviour change that can surprise; it is first on [the changes page](../guide/changes.md).
- No tracked file changes shape. The pointers are local.

## 5. Deferred, with reasons

**Citations rendered as links (`rendering = "links"`, a `render` table per kind).** A delivery's checks judge the payload that will be posted, and the candidate is bound to those bytes. A link rendering therefore changes what is checked and what is bound, and it needs answers this release did not have time to prove or test well: where a repository's web address comes from (the KB's repository map is local; a tracked map is a new file), what an unmapped repository or a kind without a template renders as (stripping silently would post a claim with no source; refusing blocks the post), and whether deny lists run before or after rendering (a rendered link can contain the internal host a deny list exists to catch). Until then `strip-citations` is the outward rendering.

**Recorded verdicts or a checker override for `code`.** `kb stale --record` commits what a declared checker said, and `verdict::recorded` (proved) decides when such a receipt stands in. Extending that to the built-in `code` kind, which is checked from local clones, changes a decision: on a machine with the clone, which wins, the local check or a newer committed receipt; on one without, how old a receipt may be before `repo-not-local` is the honest answer. That needs its theorems first. Today a machine without clones reads `unverifiable: repo-not-local`, which is accurate.

**Batch `observe --from` and `record --from`.** `gate cover --from` shows the shape (JSON lines, one write plan, all rows or none). The undecided part is evidence: an observation says a look happened in this run and how it went, stamped by the clock. An imported one happened elsewhere, earlier, under another tool's rules. It must carry its origin and original time and must not ground `supported` as a look made in the run would, or an import mints evidence. That marking and its effect on `ground` are decision logic to design and prove; the verbs are small once it is settled.

**A `repos` key on a gate definition.** Plans cover the reported need. A selector on the gate raises: what a plan step that names a gate out of scope for this repository owes (nothing would let a step be met by exclusion; "undefined" refuses a template that is right elsewhere); whether `repos` joins the definition digest and so moves every pin when edited; and what `gate run` answers there. Guard rules already take `repos`.

**Pruning pointers.** Session pointers accumulate, one short file per session that made a run current. Nothing removes them; a rule for when one is dead (its run closed, its session not seen for some time) would be a decision made from modification times, which the product avoids elsewhere.

**A built-in lookup hook, a transcript-reading requirement** ("a guideline was loaded this session"). The first is a recipe by design. The second is not something Writfold can observe; the recipe is a checklist item or an observation.

## 6. What was checked

| Claim | Found at `4f73917` |
|---|---|
| One current-run pointer per clone | Yes: `runs::current(state)` reads `<git dir>/writfold/current-run`; fifteen call sites read it; `write::execute` sets it after every `checkpoint` |
| The guard hook cannot be given a run | Yes: `guard::require::gather` calls `runs::current_of`; `guard status` has no `--run` |
| The hook has the session ID; verbs do not | Yes: `session_id` is read for drafts and marks; `$WRITFOLD_SESSION` was read only by `propose`, and nothing exported it (listed under "Not built" in the hooks guide) |
| A guarded command is judged where it acts | Yes (`Place::At`), so the work tree of that directory is available to the pointer read |
| `find a b c` is a batch | No: one phrase. A lone `find` on 3,000 records takes 48 to 92 ms with process start (validation guide) |
| A plan narrows `gates.met` | Yes (`plan::owed`, `guard/require/gates.rs`); no guard test pinned it |
| `plan repin` | Not built; the kernel fact `repinned` and the `repinned=` count existed |
| The TLA+ models include the current-run pointer | No: neither `RunGraph.tla` nor `KbWrite.tla` mentions it |
| `CLAUDE_ENV_FILE` | Available to `SessionStart` hooks; its lines run before each Bash command (Claude Code hooks reference, read 17 November 2026). Not exercised in a live session |

## 7. State of the work and order of implementation

Recorded on 17 November 2026 when implementation was handed over. Nothing was pushed, tagged or published, and no gate receipt exists for any of it.

**Branch `session-runs`** (the main checkout, at `4f73917`, no commit made): everything of sections 2.1 to 2.5 is in the working tree, uncommitted.

- Kernel and proofs: `crates/writfold-core/src/pointer.rs`, `start_from` and `lib.rs`; `proofs/Writfold/Pointer.lean`, `proofs/Writfold.lean`, `proofs/Audit.lean` (ten claims), `proofs/mutants.py` (six mutants), `proofs/Extracted/WritfoldCore.lean` refreshed after `cargo fmt`. Observed in a scratch container: no drift, `lake build` clean, the axiom audit `584 theorems, 308 required claims`, each of the six new mutants rejected by exactly its named theorems, and 139 of the 195 mutants calibrated with no failure when the run was stopped. **The full proofs gate has not passed on this tree.**
- Wiring: `writfold-kb` `seat.rs`, `hook/export.rs`, `verbs/find_each.rs` (new) and the readers and writers listed in [kb-v1-design.md §48](kb-v1-design.md#48-a-current-run-per-session-and-work-tree-find---each-as-built-17-november-2026-v060-dev6); `writfold-app` `kb/{hook,lists,mod,verbs,writes}.rs`.
- Tests: `crates/writfold-kb/tests/{seat_runs,find_each}.rs`, `crates/writfold/tests/kb_session_runs.rs`, `guide_toml.rs` (the `adopting` page). Observed on the host: these pass; `cargo fmt --check`, both clippy runs and the xtask `quality` check were clean; the `writfold`, `writfold-app`, `writfold-kb` and `writfold-tui` suites passed once before the last two edits (the library no longer reads `$WRITFOLD_SESSION`; the `guard status` note was reworded) and were re-run only in part after them.
- Documentation: the guides `hooks`, `kb-layout`, `feeding-back`, `gates`, `watch`, `json`, `assurance`, `README`, the new `adopting` and `changes`; `aeneas.md`, `kb-v1-design.md` §48, `mission.md`; the plugin's skill and hook description. They say "built in v0.6.0-dev.6", which becomes true only at the release. `aeneas.md`'s mutant and theorem totals were not updated (they wait for the merged tree).
- Not started: the version bump (`Cargo.toml`, `Cargo.lock`, README's pre-release line and its "What works today" and "Planned" lines), the release notes (a draft is kept outside the repository), the website.

**Branch `repin`** (worktree `../writfold-repin`, clean, two commits on `4f73917`: `5ee81da` kernel and proofs, `4b7f011` the verb, its tests and documentation): `plan repin`, written by a second agent and stopped before its gate finished.

- Kernel `crates/writfold-core/src/policy/repin.rs`: `in_force` (the newest pin entry that applies) and `repin_check` (refusals in the order closed, an edited pin, unpinned, nothing moved, a gate that does not load, a lock list that would reopen a skipped step). Theorems in `Policy.lean`, all audited: `in_force_loop_spec`, `in_force_spec`, `in_force_none_iff`, `repin_check_spec`, `repin_ok_iff`, `repin_unmoved_refused`, `repin_closed_refused`, `repin_keeps_skips`. Eight mutants.
- Reported by that agent, not re-checked here: no drift, `lake build` clean, the audit `579 theorems, 306 required claims`, its eight mutants each rejected by exactly their theorems, 67 of 197 mutants calibrated with no failure when stopped; `cargo fmt`, both clippy runs, the xtask check and the four crates' tests clean. **The full proofs gate has not passed on this branch either.** A paused container `wf-repin-gate` holds that run (`docker unpause` resumes it, `docker stop` discards it).
- Three things in it go beyond [policy-spec 2.5](policy-spec.md#25-policy-pins-and-drift-items-11-12) and need a reviewer's decision: a new optional `steps` key in a repin pin (a done step and an open step naming one gate at different digests cannot both read `same` from one digest per gate; an older binary ignores the key and may still read such a run `moved`); three refusals the specification does not list (an edited pin, a gate that does not load, a stricter `unskippable` list that would reopen a step the run already skipped); and the required file's pin in force becoming the newest pin that carries one, where it was the first. A repin exits 2 while a done step is still stale: it revives no evidence.
- Left for the merge: README's "Planned: `plan repin`" line; `kb-layout.md` if it is to document `steps`, `reason` and `by`; one refusal path with no end-to-end test (a done record that does not name its step's gate, reachable only by editing a record by hand).
- It will conflict with `session-runs` in `proofs/Extracted/WritfoldCore.lean` (regenerate), and may in `proofs/Audit.lean`, `proofs/mutants.py`, `docs/writfold/aeneas.md`, `docs/guide/{assurance,json}.md` and the usage text in `crates/writfold-app/src/kb/mod.rs` (244 lines on `session-runs`, 239 on `repin`, budget 250).

**Order.**

1. On `session-runs`: commit the kernel and proofs alone, then the wiring, tests and documentation.
2. Review `repin`; merge it into `session-runs`; run `cargo fmt`, refresh `proofs/Extracted/`, and correct the totals in `aeneas.md` and the `repin` line of `docs/guide/changes.md`.
3. Run the whole proofs gate on the merged tree. It must pass before the merge to `main`.
4. Host checks: `cargo fmt --all --check`, both clippy runs, the xtask `quality` check, the four crates' tests.
5. Merge to `main`; bump the version; from `main`, `git ls-files -z | xargs -0 touch`, then the quality and unit gates; receipts in an empty commit.
6. Smoke-test the musl binary on `resume --here`, `find --each`, `plan repin` and the hook's export before tagging; search the diff for anything that names a private setup; tag, publish, then the website.

**What cost time.**

- A parameter named like its module (`pointer: &Pointer` in `pointer.rs`) translates to Lean that does not elaborate (`Variable name must be atomic`). Name it otherwise.
- `proofs/mutants.py` needs the exact set of failing theorems per mutant, and a mutant of a small function fails every theorem that unfolds it. Run the mutant first, then write the set it reports.
- One full run of the proofs gate calibrates 195 mutants one after another, about 20 seconds each on an idle machine and several times that under load. `python3 proofs/mutants.py NAME…` takes names, so the calibration can be split across processes in one container; the pass condition is unchanged.
- Two proofs runs and a test run at once put the machine at a load of 60 to 80 on 32 cores. Run one at a time.
- Budgets: `crates/writfold-kb/src/lib.rs` is at the 400-line module budget (a new verb's argument list went into its own module); `writfold-app` `kb/args.rs` (249), `kb/writes.rs` (248) and `kb/mod.rs` (244) are near 250; a test function over 80 lines fails the same check.
- The library must not read `$WRITFOLD_SESSION`: once the hook exports it, `cargo test` inside an agent session would run every in-process test as that session. Only the binary reads it (`writfold_kb::run_as`).
- `writfold watch` parses the first note of `guard status` for the run ID, so that note's wording is fixed; new information goes in a second note.
- A JSON example in a guide must be real output: build the binary and run it on a scratch copy of `fixtures/kb/basic`.

## 8. Part B: attached stores for disposable evidence

A second request, queued after Part A and not part of its release. Design only.

### 8.1 The need

Agents produce material worth citing and not worth curating: data pulls, probe scripts, charts, raw research, review notes. A common arrangement keeps it in a companion Git repository beside the KB, cited with ordinary `code` citations. The companion is never curated: files are added, superseded and sometimes reorganised. What a record relies on is the content **at the cited revision**, not what is at that path now.

Checked at `4f73917`: a clone is found through the local map `<git dir>/writfold/repos`, then `$WRITFOLD_KB_REPOS`, then a `repo=` that is a path, then a `file://` URL (`stale/repos.rs`); the map is local and hand-written, so every new clone and every second machine rebuilds it. Freshness is judged at `origin/HEAD`, else `HEAD` (`repos::target`): the cited blob at the cited path there is `fresh`, the blob elsewhere `moved`, otherwise `changed` or `gone` (`stale/code.rs::settle`); the cited `rev` is read only when the token has no `blob=`. Nothing links material back to records, and a run holds no pointer to what it produced. So in a store that is reorganised, sound evidence reads `moved` or `gone`, and the evidence rule (only fresh citations ground) then refuses it.

### 8.2 Opinion

**Keep Git as the backing store.** A citation token already names a commit and a blob, so a pin needs no new format; any Git remote makes the store portable between machines with the commands the KB itself is synced with; Writfold already reads Git natively and adds no dependency, no server and no network code. A content-addressed directory (files named by digest, copied by any sync tool) is the serious alternative: pins are simpler and history cannot be rewritten, but it needs its own transport, has no notion of "a fresh clone has this", and is a second format to specify, lint and test. Git's weakness is large binary material; the answer for now is to keep such files out, or use a partial clone, which Writfold already detects. The kernel below takes "does the pin still yield the cited bytes" as facts, so a second backend can be added later without changing a decision. It is not proposed now.

**Pinning is declared per store, not written per citation.** The requester asked whether a token should carry something like `pin=1`. No: (1) the rule that judges evidence belongs to the policy a team reviews, like a gate's `accept`, not to whoever writes the citation, who would otherwise choose the weaker rule for their own evidence; (2) citation tokens are a format this project preserves, and existing tokens should change meaning by one reviewed declaration, not by being rewritten; (3) for an ordinary code repository, pinned validity is wrong (the code moved on and the decision may no longer hold), and a per-token switch invites it there.

**This is core scope, not a source checker.** A checker is for kinds Git cannot check. This is the built-in `code` kind, which is never handed to a checker, and three of the four asks (declaration, backlinks, artifacts) are nothing a checker does.

**Snapshotting outside material into the store: offer it, never force it.** The idea is sound for one purpose and wrong for another, and the design must keep them apart:

- A snapshot answers *what did this record rely on*. It is stable by construction and is the right evidence that a look happened and what it showed.
- It does not answer *is that still true*. A snapshot of a ticket or a page is never stale, which is exactly why it cannot say the ticket was closed since. That question stays with `ttl` and checked kinds.
- Writfold has no network code and will not fetch. The bytes come from the agent, so a snapshot proves "these bytes were stored then", not "the source said this". Its provenance is a statement, as an observation's is.

So the recommendation is a verb that stores what the agent hands it and pairs it with the look: `observe … --snapshot FILE` writes the bytes into an attached store with their provenance and records the observation citing them. A team that wants every look at an uncheckable source to keep its bytes says so in its own policy (a checklist item that takes only observations with a snapshot); Writfold does not make it a built-in rule, for the reasons a built-in research rubric was declined, and because copying outside material wholesale has size and licensing costs only the user can weigh. One constraint shapes the verb: a pin needs a commit, and Writfold never commits. Version one writes the file, prints the two commands that commit it and mint the token, and refuses to cite an uncommitted snapshot. Letting Writfold commit in a store it is told it owns would be a new write protocol with crash points, which needs its own TLC model first.

### 8.3 Theorems

The decisions are a new kernel, `writfold_core::store`, translated and proved before anything calls it. Each row is a theorem to state first; names are proposals.

**Which rule judges a citation** (`basis(declared, pinned_store, has_rev) -> Head | Pin`).

| Theorem | Statement |
|---|---|
| `basis_pin_iff` | the pin judges exactly when the citation's repository is a declared store whose freshness is `pinned` and the token carries a `rev` |
| `basis_head_default` | a repository that is not declared, or a store declared without `pinned`, is judged at head: every existing KB reads as before |

**The pinned verdict** (`pinned(commit, anchored, blob) -> Verdict`, where `commit` is present or absent in the clone, `anchored` says some ref reaches it, and `blob` is what `rev:path` holds: the cited bytes, other bytes, or nothing).

| Theorem | Statement |
|---|---|
| `pinned_fresh_iff` | fresh exactly when the commit is present, some ref reaches it, and `rev:path` is the cited blob |
| `pinned_behind_unverifiable` | an absent commit is `unverifiable`, never `gone` and never fresh: a clone that is behind is not evidence of loss |
| `pinned_unanchored_gone` | a commit no ref reaches is `gone` even while its objects are still on disk: a fresh clone would not have it, and it is one garbage collection from unreadable |
| `pinned_other_bytes` | a present, anchored commit whose `rev:path` is absent or holds other bytes is `gone`: the token never described that commit |
| `judge_pin_ignores_head` | under the pin, the verdict is the same for every state of the path at head (moved, changed, deleted) |
| `judge_head_unchanged` | under head, the verdict is `verdict::observed` of the head facts, for every state of the pin |
| `judge_mark_wins` | a citation marked stale by hand is stale under either rule (`verdict::with_mark`, as today) |

**Where a store is found** (`resolve(mapped, searched, hinted, origin_matches) -> Found(via) | Mismatch | Missing`).

| Theorem | Statement |
|---|---|
| `resolve_order` | the local map wins whenever it names a clone; then a search root; then a committed path hint; missing exactly when none names one |
| `resolve_local_wins` | a clone the local map names is used whatever its origin (the machine's owner said so), and is flagged when its origin is not the declared one |
| `resolve_committed_needs_origin` | a clone found by a committed hint or a search root is used only when its origin is the declared URL; otherwise the answer is `Mismatch`, and a mismatch never grounds evidence |
| `resolve_undeclared_unchanged` | for a repository with no declaration the answer is today's rule |

**Run artifacts** (`artifact(declared, attached, full_rev, blob_found) -> Admit | Refuse(reason)`).

| Theorem | Statement |
|---|---|
| `artifact_admit_iff` | a pointer is recorded exactly when the store is declared, its role is `attached`, the revision is a full commit ID, and `rev:path` was read when it was recorded; refusals in that fixed order |
| `artifact_is_a_citation` | an artifact's standing on every later read is `judge` of its token: there is no second validity rule |

**Backlinks** (`claim(attached, matches) -> Listed | Ambiguous | Unknown`).

| Theorem | Statement |
|---|---|
| `claim_listed_iff` | a directory is listed under a record exactly when it is in an attached store and its `subject` names exactly one record |
| not a theorem, a type | a claim is not among the references `ground` takes, so material that names a record can never support it |

Each function gets at least one mutant: pin judged for an undeclared repository; head consulted under the pin; an absent commit read as `gone`; an unanchored commit read as fresh; a committed hint used despite another origin; the map losing to a hint; an artifact admitted for a short revision; a claim listed when two records match.

**Tested, not proved:** reading the store files and their diagnostics; `~` in a path hint; normalising a remote URL before the proved byte comparison; the Git reads (is the commit present, does a ref reach it, what `rev:path` holds), batched as one `rev-list --all` and one `cat-file --batch-check` per store; reading `README.md` front matter from a store's tree; the index tables; rendering and JSON.

### 8.4 Design

**S1, declaring a store.** One file per store, like every other definition (the definition files take top-level keys only, so one shared file with tables does not fit):

```text
# .writfold/stores/scratch.toml
schema = "writfold-store/1"
name = "scratch"
url = "<the remote every machine clones from>"
role = "attached"          # cited, never curated
freshness = "pinned"       # the default for role = "attached"; "head" keeps today's rule
paths = ["~/src/scratch"]  # optional hints, tried in order on each machine
description = "probe scripts, data pulls and review notes"
```

`name` is what citations carry in `repo=`. `kb repos` gains one row per declared store: where it resolved and by which rule, `mismatch` with both origins, or `missing` with the `git clone` line to run (Writfold still never clones). `kb lint` reads the files. `kb policy` lists them and includes them in `identity`, since `freshness` changes what counts as evidence. The local map stays and still wins, so nothing a user has today stops working.

**S2, pinned freshness.** For a citation into a pinned store, `stale` asks the pin instead of the head. The verdict stays in the existing vocabulary: `fresh`, shown with its basis (a `pinned` flag in text, `"basis": "pin"` in JSON). A new verdict word was considered and rejected: every proved consumer (`ground`, checklist cover, the delivery check on cited claims) takes "fresh" to mean "the bytes the record relied on are the ones cited and can be read", which is exactly what the pin establishes in a store declared as never curated; a new word would change all of them to say the same thing. What the path looks like at head is not part of the verdict and is not reported in version one.

**S4, run artifacts.** `checkpoint --artifact <store>@<sha>:<path>` (repeatable) resolves the store, reads the blob at that revision and writes one create-new file `runs/<run>/artifacts/<time>-<hex>.json` holding an ordinary `code` citation token and the checkpoint it came with, through the existing write protocol and in the same write plan as the checkpoint entry. `resume` shows `artifact:` rows with each token's verdict from the verdict cache. The run sidecar and the manifest are not touched (the manifest's shape is fixed for compatibility). One file per call under the existing protocol adds no crash point, so the TLA+ models are unchanged.

**S3, backlinks.** In an attached store, a directory's `README.md` front matter may carry `subject:` (a record ID or a run ID). `kb context <id>` and `kb uses <id>` list such directories as `material` rows, each flagged as the store's own claim. They are read from the store's target commit, never from its work tree, and kept in the index keyed by that commit; the index pin gains each declared store's target commit, so a store that moved rebuilds the table. This is the one part with an architectural cost (the index would depend on a second repository's state), which is why it is last.

**The snapshot verb** ([8.2](#82-opinion)) comes after S1 and S2 and is specified then.

### 8.5 Order and scope

1. The kernel `store` with `basis`, `pinned`, `judge` and `resolve`; translation; proofs; mutants; the proofs gate.
2. S1: the store file, its parser and diagnostics, `kb lint`, `kb repos`, `kb policy`.
3. S2: `stale` judging by `judge`; the basis in text and JSON; the verdict cache keyed with the basis.
4. Release: S1 and S2 are what the request needs, and they stand alone.
5. S4, with `artifact` proved first; then S3, with `claim`; then the snapshot verb. Each its own release.

### 8.6 Open questions for the owner

- **An unanchored pin.** Proposed: `gone` ([8.3](#83-theorems)). The softer reading is fresh with a warning. The strict one is proposed because the store is meant to be portable, and an unanchored commit is not.
- **A store whose declaration changes.** `freshness` moving from `head` to `pinned` makes old `moved` evidence fresh. Proposed: the declaration is in `kb policy`'s identity and is not part of a run's pin in version one.
- **The default.** Proposed: `role = "attached"` implies `pinned`. The alternative is to require `freshness` to be written, so that the weaker rule is never implied.
- **Writfold committing in a store it owns.** Not proposed. If wanted for the snapshot verb, it is a write protocol to model before it is built.
