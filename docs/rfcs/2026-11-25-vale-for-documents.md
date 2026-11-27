---
rfc: "2026-11-25-vale-for-documents"
title: "Vale for documents: a built-in check with a pinned, bound profile"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-25
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/117
milestone: "v0.6.0-dev.21"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Vale for documents

Issues: #117 (the check), #131 (kernel), #132 (profile and digest), #133
(stand-in, tools image, conformance test), #134 (example and docs). Owner
questions: #136.

This amends one position of
[2026-11-25-prose-rules-and-advice](2026-11-25-prose-rules-and-advice.md):
"Vale: an optional recipe later (#117)". That RFC reasoned from short
delivery texts. The owner has since said that Vale is already in use at
work and that delivery modes can be documents: RFCs, design docs, guides,
long reports. Everything else in that RFC stands. `prose.rules` remains the
check for short texts and is not changed here.

## Proof obligations

A new kernel, `writfold_core::vale`, decides what one Vale run means. It
reuses `prose::Weight`, `prose::verdict` and `gate::Ending`.

- `weigh(level, from)` turns a Vale severity (`Suggestion`, `Warning`,
  `Error`) into `Advice` or `Blocking` under the mode's threshold
  (`Never`, `Error`, `Warning`, `Suggestion`).
- `usable(ran)` says whether a run can be read at all: the executable is
  the pinned version, the output is whole, and the exit status is 0 with no
  error-level alert or 1 with at least one.
- `rules(enabled, canary)` is the number of enabled checks, or 0 when the
  canary drew no alert.
- `settle(ran, found)` is `Unknown` unless the run is usable, and otherwise
  `prose::verdict(rules, found)`.

`proofs/Writfold/Vale.lean` proves:

- `weigh_spec`, and from it:
  - `weigh_never`: with `Never`, every alert is advice;
  - `weigh_error_blocks`: with any other threshold, an error blocks;
  - `weigh_mono`: if a severity blocks, every higher severity blocks.
- `usable_spec`, `usable_exit` (a usable run exited 0 or 1) and
  `usable_exit_agrees` (0 with no error-level alert, 1 with some).
- `settle_unusable_unknown`: a missing executable, another version, a
  signal, a timeout, a start failure, exit 2 or higher, an exit status that
  disagrees with the alerts, and cut-off or unreadable output are all
  `Unknown`. None is a pass and none is a fail.
- `settle_no_coverage_unknown`: no enabled check, or a canary that drew no
  alert, is `Unknown`.
- `settle_pass_iff` and `settle_fail_iff`: a pass is exactly a usable run
  with coverage and nothing blocking; a fail is exactly a usable run with
  coverage and something blocking.
- `settle_advice_never_decides` and `settle_perm`, lifted from
  `Writfold.Prose`: advice and order are not inputs.
- `settle_agrees`: a receipt written from `settle` satisfies
  `prose::agrees`.
- `pass_stricter_pass`: a document that passes under a stricter threshold
  passes under every laxer one.

`rules_spec`, `settle_spec` and `countB_weigh_zero` are the lemmas the rest
rests on.

Mutants (each names only the spec theorem whose proof unfolds the mutated
function; the corollaries follow from it):
- `vale-absent-usable`, `vale-other-version-usable`, `vale-exit2-usable`,
  `vale-exit-disagrees-usable` and `vale-cut-usable` break `usable_spec`;
- `vale-no-canary-pass` breaks `rules_spec`;
- `vale-never-blocks` and `vale-warning-skips` break `weigh_spec`;
- `vale-unusable-fail` breaks `settle_spec`.

**Tested, not proved:**
- probing `vale --version` and reading its one line;
- running the process, its isolation and its limits;
- parsing Vale's JSON, and the severity strings;
- reading `StylesPath` from `.vale.ini`;
- the tree inventory and its digest;
- receipt encoding, ordering and rendering.

The staleness decision is the existing comparison of obligation digests.

## Why a built-in check, and not a script gate

| Need | Script gate | Built-in `prose.vale` |
|---|---|---|
| A long document's report | keeps the last 64 KiB of output | its own cap (8 MiB); over it is `unknown` |
| Stale when a style file changes | digest covers the gate's own files | digest covers the whole profile tree |
| Missing or wrong Vale | whatever the wrapper does | `unknown`, proved |
| Vale's check names (`House.Hedges`) | rule names must be lower case | kept verbatim |
| Severity as advice or blocking | exit status only: all or nothing | per alert, by threshold |

Vale stays an external executable that the user installs. Writfold does not
link it, download it or run `vale sync`. It is not a companion process: it
runs once per `deliver check` and exits. A KB with no mode that lists
`prose.vale` never looks for it.

## What Vale does (v3.28.1, read from its source, not run)

- `vale --output=JSON --counts <files>` prints
  `{"files": {"<path>": [alert, …]}, "counts": {"<Style.Rule>": n, …}}`.
  A file with no alert is absent from `files`. `counts` has an entry, zero
  included, for every check Vale loaded. `--counts` exists since 3.20.0.
- An alert has `Check`, `Severity` (`suggestion`, `warning`, `error`),
  `Line`, `Span` (begin and end within the line), `Message`, `Match`,
  `Description`, `Link`, `Action` and `Suggestions`.
- Exit 0: no error-level alert. Exit 1: at least one. Exit 2: a runtime or
  configuration error, or a file that could not be linted; the error is
  JSON on standard error (`Code`, `Text`, `Path`, `Line`, `Span`).
- The global config and the default styles path are loaded in addition to
  `--config`. `--no-global` (since 3.2.0) turns both off.
  `VALE_CONFIG_PATH` and `VALE_STYLES_PATH` are read from the environment.
- `counts` lists every loaded check, whether or not the file matched a
  section of the config. A config whose sections match nothing therefore
  looks like a clean run. The canary closes that gap.
- Only `vale sync` uses the network.

## Decision

1. **Profile.** `.writfold/deliveries/vale-<name>.toml` is Writfold's pin:

   ```toml
   schema = "writfold-vale-profile/1"
   name = "house"
   version = "3.28.1"
   canary = "This sentence is very, very simple and has a TODO."
   ```

   `version` is exact and at least 3.20.0. `canary` is one line that must
   draw at least one alert. `timeout_seconds` is optional (default 60).
2. **Native tree.** `.writfold/vale/<name>/` holds the user's own
   `.vale.ini` and styles directory, unchanged. Editors and CI use the same
   files with `vale --config`. Adopting an existing setup is a copy. The
   config's top-level `StylesPath` must be relative and stay inside the
   tree; otherwise the mode is refused.
3. **Mode keys.** A mode that lists `prose.vale` in `checks` names its
   profile with `vale = "<name>"`: required with the check, refused without
   it. `vale_block` is `error` (default), `warning`, `suggestion` or
   `never`. Built-in modes do not list the check.
4. **Digest.** The obligation digest covers the profile's name, the pinned
   version, `vale_block`, the canary and an inventory of every file in the
   tree (path and sha256, sorted). A symlink, an unreadable entry, more
   than 2,000 files or more than 8 MiB refuses the mode. `status` hashes
   files and never runs Vale.
5. **Run.** The payload is copied to `document.md` in a fresh private
   directory with the canary as `canary.md` beside it. One run:
   `vale --no-global --config <tree>/.vale.ini --output=JSON --counts
   document.md canary.md`, in that directory, with every `VALE_*` variable
   removed and `HOME` and `XDG_CONFIG_HOME` set to it. `vale --version` is
   probed first. Writfold does not pass `--minAlertLevel` or `--no-exit`.
6. **Verdict.** `vale::weigh` for each alert on the document, then
   `vale::settle`. Alerts on the canary count only as coverage.
7. **Receipt.** No format change. Blocking alerts go in `result.findings`,
   the rest in `result.advice`, each as line, rule (Vale's `Check`) and
   detail (`column N: severity: message`). The first 20 of each are kept,
   with exact totals. The note has the version, the number of enabled
   checks, totals by severity and the five most frequent rules.
8. **Showing.** `deliver check` and `status` print the totals, the first
   findings and the native command that lists every alert. Each cause of
   `unknown` has its own `next:` line. Advice never appears in `next:`.
9. **Scope.** Markdown only. A document mode is a user mode without
   `length.bounded`; candidates stay under the existing 1 MiB limit.

**What it does not do:** install or update Vale, run `vale sync`, ship
third-party style packs, keep Vale's full report (#128), bind the inputs of
script gates (#116), lint other formats, fix text, or give a quality score.
The receipt says that this executable version with these files reported
these alerts on these bytes. A version string is not proof of which binary
ran.

## Acceptance gates

| Gate | Evidence |
|---|---|
| proofs | `Writfold.Vale` builds; the 9 mutants calibrate; the axiom audit passes |
| unit | `deliver_vale_profile.rs` (each bound input stales a receipt; containment and inventory faults refuse) |
| unit | `deliver_vale.rs` against the stand-in (every cause of `unknown`; the threshold; advice never decides; argv, `HOME` and the private directory; a 3,000-line document with 2,000 alerts) |
| unit | `deliver_vale_real.rs` against the pinned Vale in the tools image (JSON shape; exit 0, 1, 2; a planted global config changes nothing; findings equal a native run). An absent Vale fails under `WRITFOLD_REQUIRE_VALE=1` |
| unit | `deliver_example.rs`, `kb_init.rs`, `guide_toml.rs` for the design-doc example |

## As built

- The stand-in in `fixtures/vale/` takes its behaviour from a first line of
  the document (`<!-- standin: MODE -->`), because the run removes every
  `VALE_*` variable and library tests cannot set environment variables.
  Tests select it with `writfold_kb::deliver::vale::use_program`; the
  binary reads `WRITFOLD_VALE`.
- Planted `VALE_CONFIG_PATH`, `VALE_STYLES_PATH` and a global config are
  exercised by the real-Vale test only. Library tests cannot set
  environment variables, so `deliver_vale_real.rs` reruns its own test in a
  child process that inherits the planted values.
- Run against the real Vale 3.28.1 (27 November 2026), the JSON shape, the
  exit statuses and the exit-2 error object on standard error are as read
  from its source. The tools image (`writfold-tools@sha256:5198a49f…`) adds
  the release tarball, checked against its published sha256, and
  cargo-tauten 0.2.0 (#100) as two new layers over the previous image; the
  distro packages are unchanged. `scripts/unit_tests.py` sets
  `WRITFOLD_REQUIRE_VALE=1`.
- #132's golden line is `deliver_receipt_golden.rs`: its golden file was
  written at the last commit without `prose.vale` and matches on main.

## Decision log

- 2026-11-25: written after the owner asked for Vale support for document
  delivery modes, and accepted by default under the standing acceptance
  (AGENTS.md). It amends the "Vale later" position of
  2026-11-25-prose-rules-and-advice. The owner questions are in #136, and
  the work proceeds on their defaults.
- 2026-11-27: the owner gave the go-ahead for the tools-image rebuild
  (#136 question 7). The real-Vale conformance test and the image landed;
  the RFC is implemented.
