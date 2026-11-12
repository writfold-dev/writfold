# Gates

> **Status:** built in v0.3.0-dev.1, except the sections marked *Planned* (instruction gates, run-record checks).

A gate is one small check with a path scope. `gate run` runs its command and keeps a receipt pinned to the exact bytes checked. `gate status` then says, on every read, whether that evidence still holds. Gates report; they block nothing.

## A definition, every key

One file per gate, `.writfold/gates/<id>.toml`. It lives in the KB root (shared by every project) or in the repository it checks (versioned with the code). The same ID in both places is an error.

```toml
schema = "writfold-gate/1"         # required
id = "unit"                        # required; a slug equal to the file name
version = 1                        # required; your own revision number
class = "mechanical"               # required; mechanical | observed | attested
argv = ["cargo", "test", "--locked"]   # required for a command gate; never a shell string
workdir = "."                      # optional; relative to the repository's top
timeout_seconds = 600              # required for a command gate; 1 to 86400
scope = ["src/**", "Cargo.toml", "!src/**/*.md"]   # required; see below
description = "the unit tests"     # optional; one line
```

- **`argv`** is executed directly, with stdin closed. A first element containing `/` is resolved against `workdir`; otherwise `PATH` finds it. `{artifacts}` is the only placeholder: a fresh local directory for the command's own output files. Any other `{name}` is refused.
- **Unknown keys are refused,** so a misspelt field never silently changes a gate. The files use a small TOML subset: top-level `key = value`, comments, strings, integers and arrays of strings.
- **Changing** the command, working directory, timeout, class, version or scope makes earlier receipts stale. Changing comments, layout or the description does not.

### Classes

| Class | What it is | Met by |
|---|---|---|
| `mechanical` | a deterministic check of the repository's bytes: build, tests, lint | a passing receipt from `gate run` |
| `observed` | a command whose result also depends on the world: a service answering | a passing receipt from `gate run` |
| `attested` | a statement someone must make; it runs nothing | `gate attest` |

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
  "recorded": "2026-11-12T21:10:36Z",
  "digest": "sha256:f4dc7a4b…"
}
```

Receipts are a log: a rerun adds a file. Only `gate run` writes them. A hand-edited outcome, a file without the right digest and a real receipt copied under another gate's name all read as `[invalid]` and count for nothing. Output tails (the last 64 KiB of stdout and stderr) stay local and untracked; the receipt holds only their sizes and digests.

## Standing

| Standing | When | Met? |
|---|---|---|
| `open` | no receipt for this repository in this run | owed |
| `satisfied` | the newest receipt about the current definition and bytes passed | met |
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

Today every gate defined for a repository is owed by every run on it.

## Planned: instruction gates

A gate may name an `instructions` file. Its digest becomes part of the definition, so editing the instructions makes earlier evidence stale. A gate may also ship its own scripts, reached through a `{gate}` placeholder. Not built: today's parser refuses the `instructions` key and any placeholder other than `{artifacts}`.

<!-- planned -->
```toml
schema = "writfold-gate/1"
id = "wording"
version = 2
class = "attested"
instructions = "wording.md"        # planned: read before attesting
scope = ["README.md", "docs/**"]
```

## Planned: run-record checks

A mechanical gate kind evaluated over the run's own records instead of code. Two examples from the mission:

- `research.backfilled`: the step wrote at least one KB record, citation or retrieval receipt.
- `run.reflected`: the run holds a reflection entry, and either a proposal or an explicit "no proposal".

A run-record check can be paired with an attested gate whose instructions say what a good reflection covers: the check shows that something was written, the statement that someone read it against the guideline. Together with required steps ([run-graph](run-graph.md)) this is how "every research step backfills the KB" would be expressed. None of it is built.

## What is claimed

- **Mistakes, not adversaries.** A gate catches a forgotten check, a check run on other bytes, and a "tests pass" that is stale. A careful forger can still write a receipt that reads as valid; nothing is signed.
- **Git is the tamper evidence.** Definitions, receipts and statements are tracked text, so a weakened definition or an edited receipt shows in a diff.
- **A gate does not stop anything.** An agent can skip `gate run`. `gate status` and `resume` will show the gate as open.
