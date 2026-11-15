# Guard conformance cases

Language-neutral statements of what Writfold's guard hook decides. Each case is the hook's input as a harness sends it, the world it runs in (files, repositories, the KB's rules, run and gates), and the decision expected. `crates/writfold-kb/tests/guard_conformance.rs` builds every world in a temporary directory, runs the event through the real hook entry point (`writfold_kb::hook::run`) with a fixed clock, compares, and reports every failing case by its ID. It runs under `cargo test`, so the unit gate counts it. The Rust tests beside it stay; these cases say the same decisions in a form another implementation can run.

## Layout

```text
fixtures/guard-conformance/
  README.md                     this file
  <suite>/SUITE.json
  <suite>/cases/<case-id>/event.json
                          world.json
                          expected.json
                          transcript.jsonl      optional; kept for other implementations, never read
```

## Format

`SUITE.json` (`writfold-guard-suite/1`): `name`, `source` (`this repository`, or a URL), `revision` (the source commit, or null), `licence` (an SPDX identifier), `imported` (a date, or null), `converted_by` (how the cases were converted, or null), `cases` (the count, which must equal the number of case directories). Every key is required.

`event.json` is the hook's input: `hook_event_name` (`PreToolUse` or `Stop`), `cwd`, `session_id`, and as the event needs them `tool_name`, `tool_input`, `stop_hook_active`, `last_assistant_message`. Every path is written under `{root}` or `{home}`, and every string has them replaced before the event is sent.

`world.json` (`writfold-guard-world/1`):

| Key | Meaning |
|---|---|
| `home` | the home directory (`HOME` for the reader's `~` and `$HOME`), under `{root}` |
| `dirs` | plain directories to create |
| `repos` | repositories: `top`, optional `origin` (set as `remote.origin.url`), `worktrees` (linked worktrees to add), `rules` (that repository's own rules, `{id: file text}`). Each is created with one commit holding `README.md` and `src/lib.rs` |
| `kb.rules` | the KB root's rules, `{id: file text}` |
| `kb.run` | `open`, `closed` or `none`: the current-run pointer and the run's state |
| `kb.map` | the KB clone's local repository map, `{name: path}` |
| `kb.gates` | `{id, repo, standing}`: an attested gate over `src/**`, defined in the KB root; `met` files an attestation and `waived` a waiver for that repository through the ordinary verbs, `open` files nothing |
| `kb.nudged` | sessions whose Stop nudge was already given (their local draft carries the flag) |
| `kb.unreadable` | `true`: the KB's local state directory is a file, so the index cannot be opened |

The KB is the invented fixture KB (`fixtures/kb/basic`) committed to a fresh repository, with its run `run-20261101T090000Z-a1b2c3`.

`expected.json` (`writfold-guard-expected/1`): `decision` is `pass`, `deny` or `ask` for `PreToolUse`, and `nudge` or `quiet` for `Stop`. `decided_by` (the sorted IDs of the rules whose text the reason holds) and `reason_has` (substrings of the reason or the nudge) are optional. A converted case whose source expects otherwise carries `"upstream": {"decision": "…"}` and `"differs": "<why Writfold decides this way>"`.

## Rules of the suite

- Reading is strict: an unknown key, a missing file, a path outside `{root}` and `{home}` (or holding `..`), or a case the runner cannot build fails the test. There is no skipped case. A case Writfold cannot evaluate (one that needs a transcript, say) is not imported, and is listed below by its source name with the reason.
- A difference from the source's expectation is recorded, not hidden and not relabelled: `expected.json` holds Writfold's decision, `upstream` the source's, `differs` the reason, and the table below lists the case. A test fails when a case whose `upstream` differs has no `differs`, or is missing from the table.
- Provenance: synthetic data only, never a real host, user, path or command line taken from a session; a licence compatible with this repository's (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, CC0-1.0 or Unlicense), stated in `SUITE.json` and by the contributor in the commit; the source revision pinned. A suite written elsewhere is accepted by converting it into this format at import; there is no runtime adapter, and the converter is not part of the product.

## Suites

| Suite | Source | Cases | What it states |
|---|---|---|---|
| `writfold` | this repository, MIT | see `SUITE.json` | how a command is read (every row of the command table in the guide), judging the repository a command acts on, directories that cannot be told, repository rules, linked worktrees, an empty set of owed gates, `repos` and `paths` exclusions, an unreadable KB, `ask`, and the Stop nudge once per session and once per turn |

Not stated as cases, because a world here has no plan: `gates.met` over a plan that names no gate (it is a Rust test, `tests/guard_empty.rs`).

## Recorded differences

| Suite | Case | Upstream | Writfold | Why |
|---|---|---|---|---|

None yet: the only suite is Writfold's own.

## Not imported

None yet.
