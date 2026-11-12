# Gates

> **Status:** built in v0.3.0-dev.1; instruction gates, gate scripts, the Weft assessed gate, run-record checks and plan-owed gates built in v0.4.0-dev.1.

A gate is one small check with a path scope. `gate run` runs its command and keeps a receipt pinned to the exact bytes checked. `gate status` then says, on every read, whether that evidence still holds. Gates report; they block nothing.

## A definition, every key

One file per gate, `.writfold/gates/<id>.toml`. It lives in the KB root (shared by every project) or in the repository it checks (versioned with the code). The same ID in both places is an error.

```toml
schema = "writfold-gate/1"         # required
id = "unit"                        # required; a slug equal to the file name
version = 1                        # required; your own revision number
class = "mechanical"               # required; mechanical | observed | attested | run-record
argv = ["cargo", "test", "--locked"]   # required for a command gate; never a shell string
workdir = "."                      # optional; relative to the repository's top
timeout_seconds = 600              # required for a command gate; 1 to 86400
scope = ["src/**", "Cargo.toml", "!src/**/*.md"]   # required; see below
description = "the unit tests"     # optional; one line
```

- **`argv`** is executed directly, with stdin closed. A first element containing `/` is resolved against `workdir`; otherwise `PATH` finds it. Placeholders: `{artifacts}` (a fresh local directory for the command's own output files), `{gate}` and `{candidate}` (below). Any other `{name}` is refused.
- **Unknown keys are refused,** so a misspelt field never silently changes a gate. The files use a small TOML subset: top-level `key = value`, comments, strings, integers and arrays of strings.
- **Changing** the command, working directory, timeout, class, version or scope makes earlier receipts stale. Changing comments, layout or the description does not.

### Classes

| Class | What it is | Met by |
|---|---|---|
| `mechanical` | a deterministic check of the repository's bytes: build, tests, lint | a passing receipt from `gate run` |
| `observed` | a command whose result also depends on the world: a service answering | a passing receipt from `gate run` |
| `attested` | a statement someone must make; it runs nothing | `gate attest` |
| `run-record` | a check over the run's own records, not code ([below](#run-record-checks)) | a passing receipt from `gate run` |
| `assessed` | a plan step verified by Weft from cited evidence ([below](#weft-assessed-gates)) | a passing receipt from `gate run`: every required check entailed from grounded assumptions |

An attested gate has no `argv`, `workdir` or `timeout_seconds`:

```toml
schema = "writfold-gate/1"
id = "wording"
version = 1
class = "attested"
scope = ["README.md"]
description = "README follows the language guideline"
```

### Scope patterns

Patterns are anchored at the top of the repository. `*` and `?` match within one path segment. A `**` segment matches any number of segments, none included. A leading `!` excludes. A path is in scope when some pattern matches it and no exclusion does: `README.md` names only the top-level file, `**/*.md` every Markdown file.

The scope decides staleness. A receipt is about the HEAD commit plus the content of the in-scope paths, uncommitted edits included. Committing the exact bytes a receipt was taken on leaves it satisfied; reverting to them makes it satisfied again.

## Running, and the receipt

```text
$ writfold kb gate run unit
gate unit [satisfied] mechanical pass exit=0 at harbor-api@4be03f7897b6+dirty:2 scope=01cc14a55a8f paths=2 @runs/run-…/gates/unit-20261112T211036Z-f5f8fb.json:1
```

The run is `--run ID`, else the current run. The repository is `--repo DIR`, else the one enclosing the working directory, else the run's recorded worktree. Exit status 0 is `pass`, any other exit status is `fail`, and a signal, a timeout, a failure to start or a subject that moved while the command ran is `unknown`, never a pass.

The receipt is one JSON file, `runs/<run>/gates/<gate>-<UTC time>-<6 hex>.json` (fields regrouped for reading; `…` cuts digests, and the `output` block is left out):

```json
{
  "schema": "writfold-gate-receipt/1",
  "gate": "unit", "version": 1, "class": "mechanical",
  "definition": { "digest": "sha256:27977094…", "origin": "repo",
                  "path": ".writfold/gates/unit.toml", "scope": ["src/**"] },
  "run": "run-20261112T211025Z-7fe148",
  "subject": { "repo": "harbor-api", "commit": "4be03f7897b691715114a229ab1acc307f5de90f",
               "scope": "sha256:01cc14a5…", "paths": 2, "dirty": 2, "dirty_digest": "sha256:51157194…" },
  "command": { "argv": ["grep", "-rq", "with_retry", "src"], "workdir": ".", "timeout_seconds": 60 },
  "result": { "outcome": "pass", "exit_code": 0, "signal": null, "timed_out": false,
              "error": null, "subject_moved": false, "duration_ms": 6 },
  "evaluator": "writfold 0.3.0-dev.1",
  "recorded": "2026-11-12T21:10:36Z", "seq": 1,
  "digest": "sha256:f4dc7a4b…"
}
```

Receipts are a log: a rerun adds a file. Only `gate run` writes them. A hand-edited outcome, a file without the right digest and a real receipt copied under another gate's name all read as `[invalid]` and count for nothing. Output tails (the last 64 KiB of stdout and stderr) stay local and untracked; the receipt holds only their sizes and digests.

## Standing

| Standing | When | Met? |
|---|---|---|
| `open` | no receipt for this repository in this run | owed |
| `satisfied` | the newest receipt about the current definition and bytes passed (newest by the receipt's `seq`, never by its clock) | met |
| `failed` | that receipt failed | owed |
| `unknown` | that receipt is a timeout, crash or moved subject | owed |
| `stale` | there are receipts, but none about the current definition and bytes | owed |
| `waived` | a check that is not satisfied, with a current waiver | met |
| `attested` | an attested gate with a current attestation | met |

Every owed row carries a `why:` line where there is one and a `next:` line with the command to run. `gate status` and `gate lines` exit 0 only when every gate is met. `gate lines` prints the same facts as lines for a commit message:

```text
Gates for harbor-api@4be03f7897b6 (writfold kb gate, run run-20261112T211025Z-7fe148): 0 of 2 satisfied, 1 attested
unit:    stale pass exit=0 runs/run-20261112T211025Z-7fe148/gates/unit-20261112T211036Z-f5f8fb.json
wording: attested by agent: README unchanged runs/run-20261112T211025Z-7fe148/statements/wording-20261112T212106Z-32b104.json
```

## Statements: `attest` and `waive`

```text
$ writfold kb gate attest wording --statement "README unchanged" --by agent
$ writfold kb gate waive unit --reason "flaky on this machine; rerun in CI" --by owner
gate unit [waived] mechanical pass exit=0 at harbor-api@4be03f7897b6+dirty:2 scope=01cc14a55a8f paths=2 @runs/run-…/gates/unit-20261112T211036Z-f5f8fb.json:1 !waived
  waived: flaky on this machine; rerun in CI (by owner, 2026-11-12T21:47:40Z) @runs/run-…/statements/unit-20261112T214740Z-180ac5.json:1
```

A statement is a file in `runs/<run>/statements/`, bound to the same definition and bytes as a receipt, so it goes stale the same way. It is never shown as satisfied. `attest` is refused for a command gate, and `gate run` is refused for an attested gate. `--by` is required and recorded as given; Writfold cannot check it.

## Re-running a receipt: `gate verify`

`gate verify <receipt>` re-runs the receipt's recorded command at its recorded commit, in a temporary detached worktree, and answers `agree`, `disagree`, `inconclusive` or `unverifiable`. Nothing is written to the KB. An example is in [validation](validation.md).

## Which gates a run owes

A run without a plan owes every gate defined for its repository. A run with a plan ([run-graph](run-graph.md)) owes only the gates its steps name: `gate status`, `gate lines` and `resume` list those, and `plan advance` judges the current step's.

Receipts are judged in the order of their `seq` field, one more than the highest of the run's receipts of that gate on that subject when it was written; `recorded` is shown, never compared, so a clock that is wrong on one machine cannot change which receipt decides. Receipts written before `seq` existed count as 0 and keep their file-name order; two clones that wrote the same `seq` are ordered by file name.

## Instruction gates and gate scripts

A gate may name an `instructions` file, a Markdown file beside the definition that the agent reads before running or attesting the gate. Its bytes are part of the definition digest, so editing the instructions makes earlier receipts and attestations stale. `gate status`, `gate run`, `resume` and `deliver status` print a `read:` line pointing at it.

A gate may also ship its own script. `{gate}` expands to the directory holding the definition, so a gate kept in the shared KB carries its check with it; the script's bytes are hashed into the digest too. `{candidate}` is the draft file for gates that check a delivery candidate (`subject = "candidate"`). Paths that leave the definition's directory, including through a symlink, are refused.

```toml
# .writfold/gates/language-guidelines.toml: read the guideline, then attest
schema = "writfold-gate/1"
id = "language-guidelines"
version = 1
class = "attested"
subject = "candidate"
instructions = "language-guidelines.md"
description = "the message follows the language guidelines"
```

```toml
# .writfold/gates/commit-format.toml: a script shipped with the gate
schema = "writfold-gate/1"
id = "commit-format"
version = 1
class = "mechanical"
subject = "candidate"
argv = ["sh", "{gate}/commit_format.sh", "{candidate}"]
timeout_seconds = 10
description = "subject at most 72 characters, a blank second line, body wrapped at 72"
```

A script gate reports findings as `<line>: <text>` on stdout; only a clean exit 0 passes. A timeout, a signal or a draft that changed while it ran is `unknown`. A worked example is in `fixtures/kb/commit-message/`.

## Weft assessed gates

A gate of class `assessed` is met by a [Weft](https://github.com/writfold-dev/weft) assessment of a plan step instead of a command's exit status. The agent states the step's assumptions and checks in a Weft file, cites the evidence for each assumption, and runs the gate. Writfold resolves every citation, runs the pinned `weft` binary, and records which checks Weft entails and whether each rests on grounded evidence.

```toml
# .writfold/gates/ship-batching.toml: the step is verified by Weft
schema = "writfold-gate/1"
id = "ship-batching"
version = 1
class = "assessed"
weft = "{run}/plan/ship.weft"       # submitted by the agent in the run; or "{gate}/<file>.weft" beside this file
checks = ["ready"]                  # the Weft checks that must be met
weft_sha256 = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"   # sha256sum of your weft binary
timeout_seconds = 60
description = "the batched ledger writes are ready to ship"
```

- **`weft`** is `{gate}/<path>.weft` (a source shipped beside the definition) or `{run}/<path>.weft` (a file the agent writes under `runs/<run>/`). Either must stay inside its directory.
- **`weft_sha256`** pins the binary. Its path is never in the definition or on the command line: it is `$WRITFOLD_WEFT`, else `weft = "<path>"` in `~/.config/writfold/config.toml`. A binary with another digest is refused before it runs; no binary at all records `unknown`, never a pass.
- An assessed gate has no `argv`, `workdir` or `scope`: what it judges is its Weft source and the evidence the source cites. `gate attest` and `gate waive` refuse it.

The Weft file is ordinary Weft. Each assumption cites its evidence with Weft's own binding, `claim <name> = citation "<reference>";`, declared **directly above** the `assume` it supports (a citation followed by a check, or by nothing, is refused):

```weft
// runs/<run>/plan/ship.weft: plan step "ship the batched ledger writes"
review ship_batching {
    context "kb" at "run";
    profile "weft.symbolic/1";
    target change = proposal "batch-ledger-writes";

    claim unit_green = citation "gate:unit";
    assume tests_pass: passes("unit");

    claim stall_done = citation "⟦cite kind=record path=ktlo/2026/hbr-99-flush-stall.md state=done⟧";
    assume stall_fixed: fixed("hbr-99");

    check ready: passes("unit") and fixed("hbr-99");
    check rolled_back: not passes("unit");
    check deployed: deployed("production");
}
```

A reference may be:

| Reference | Grounded when |
|---|---|
| a KB citation token `⟦cite …⟧` or a locator `repo@commit:path[:a-b]` | `stale` says it is fresh |
| `gate:<id>` | the gate is `satisfied` (or `attested`, kept as attested) |
| a gate receipt of this run, by path or name | it passed, about the gate's current definition and bytes |
| an attestation of this run, by path | it is the gate's current attestation (attested) |
| a retrieval receipt, by path or name | every citation in it is fresh |
| an edit receipt, by path or name | every file it edited still holds the bytes it wrote |

An assumption with no citation, or with any citation that is not grounded, is ungrounded. Weft's report lists, for each check, the assumptions it used (every assumption declared above the check). A check **meets** the gate only when Weft's result is `entailed` and every one of those assumptions is grounded. Weft's results are kept in its own words, `entailed`, `refuted`, `undetermined` or `inconsistent-premises`; the basis is `grounded`, or `hypothetical` when any assumption it used is ungrounded. A hypothetical check is flagged `!hypothetical` and is never counted as met, whatever its result. The gate passes when every required check is met; Weft unable to finish (no binary, a timeout, an incomplete report) is `unknown`.

The output below is from `crates/writfold-kb/tests/gate_assessed.rs`, where this source is shipped as `{gate}/step.weft` for a gate named `ship` and `bin=` is the test double's digest. With `unit` satisfied and the record fresh, `ready` is entailed on a grounded basis and the gate is satisfied:

```text
gate ship [satisfied] assessed pass met=1/1 weft=f463cf9c4a50 bin=8b46349b5622 at demo-service@bcacd8cac84e @runs/run-…/assessed/ship-20261111T000100Z-….json:1
  check: ready [entailed grounded] met
  check: rolled_back [refuted grounded] (not required)
  check: deployed [undetermined grounded] (not required)
```

The same source after an in-scope file changed, so `gate:unit` is stale: Weft still entails `ready`, but from an ungrounded assumption, and the gate fails:

```text
gate ship [failed] assessed fail met=0/1 weft=f463cf9c4a50 bin=8b46349b5622 at demo-service@bcacd8cac84e @runs/run-…/assessed/ship-20261111T000100Z-….json:1 !failed
  check: ready [entailed hypothetical] not met; rests on ungrounded tests_pass !hypothetical
  check: rolled_back [refuted hypothetical] (not required); rests on ungrounded tests_pass !hypothetical
  check: deployed [undetermined hypothetical] (not required); rests on ungrounded tests_pass !hypothetical
  cite: tests_pass unit_green [ungrounded] stale @runs/run-…/gates/unit-20261111T000000Z-….json:1
  next: writfold kb gate run ship (Weft source {gate}/step.weft)
```

The receipt, `runs/<run>/assessed/<gate>-<time>-<hex>.json`, binds the definition digest, the Weft source's digest, the pinned and actual binary digests, the report's digest (the report itself stays local), every citation with what it stood on, and every check's Weft result, Weft's own basis and Writfold's basis. Reading it re-derives every basis and the outcome, so a hypothetical check relabelled grounded reads as `[invalid]`. It goes `stale` when the source changes or any cited evidence moves (`why: cited evidence moved: stall_done (…) was fresh, now changed`), and is satisfied again if the evidence returns.

## Run-record checks

A gate of class `run-record` runs no command: `gate run` judges the run's own records against the built-in requirements its `require` list names, and records a receipt like any other (exit status 0 for met, 1 for not met, with the reason as a note).

```toml
# .writfold/gates/backfilled.toml
schema = "writfold-gate/1"
id = "backfilled"
version = 1
class = "run-record"
require = ["research.backfilled"]    # research.backfilled | run.reflected
description = "the research step wrote something back to the KB"
```

- **`research.backfilled`**: since the step began, the KB's files outside `runs/` changed (a record was written or edited), a retrieval receipt was recorded in the run, or a checkpoint entry holds a citation token.
- **`run.reflected`**: a checkpoint entry starts with `reflection:`, and the run has a pending proposal, a KB record written in the step (an accepted proposal), or a line starting `no proposal`.

A run-record gate takes no `scope`, `subject`, `argv`, `workdir` or `timeout_seconds`. Its receipt binds the run-record digest of the window it judged: the current plan step's, from the event that began the step to now (the whole run when the run has no plan). Any change to the run's records in that window (a new checkpoint entry, a retrieval receipt, a KB file) makes it `stale`; once the step is done, its window is closed and the receipt keeps standing. `gate waive` sets one aside like any check; `gate attest` is refused, and `gate verify` answers `unverifiable` (there is no commit to re-run it at).

These checks show that something was written, not that it is good. Pair one with an attested gate whose instructions say what a good backfill or reflection covers: the check shows the record exists, the statement that someone read it against the guideline.

```toml
# .writfold/gates/reflection-reviewed.toml: what a good reflection covers
schema = "writfold-gate/1"
id = "reflection-reviewed"
version = 1
class = "attested"
instructions = "reflection.md"
scope = ["**"]
description = "the reflection names what went wrong and what to change"
```

## What is claimed

- **Mistakes, not adversaries.** A gate catches a forgotten check, a check run on other bytes, and a "tests pass" that is stale. A careful forger can still write a receipt that reads as valid; nothing is signed.
- **Git is the tamper evidence.** Definitions, receipts and statements are tracked text, so a weakened definition or an edited receipt shows in a diff.
- **A gate does not stop anything.** An agent can skip `gate run`. `gate status` and `resume` will show the gate as open.
