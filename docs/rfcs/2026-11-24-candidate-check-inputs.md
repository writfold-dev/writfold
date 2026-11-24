---
rfc: "2026-11-24-candidate-check-inputs"
title: "Bind candidate checks to their complete declared inputs"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/116"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Bind candidate checks to their complete declared inputs

## Summary

A candidate check is applicable only to the payload, policy and declared
checker inputs it evaluated. Add explicit local input inventories and tool
identity to candidate-gate binding. Reuse Writfold's scope, inventory, pin and
receipt mechanisms; keep backend configuration interpretation in the adapter.

This is the input-binding slice of [#119](https://github.com/writfold-dev/writfold/issues/119).
The proposal defines the required semantics; exact definition keys remain for
grooming rather than being presented as an available API.

## Motivation

At `4708ebfbdcaa`, [gate file collection](https://github.com/writfold-dev/writfold/blob/4708ebfbdcaa087e6bc02540bb83ac46fd9b612a/crates/writfold-kb/src/gate/places.rs)
hashes files explicitly named through `{gate}/...` in argv, and the
[definition digest](https://github.com/writfold-dev/writfold/blob/4708ebfbdcaa087e6bc02540bb83ac46fd9b612a/crates/writfold-kb/src/gate/def.rs) includes them.
It does not discover a configuration's transitive resources.
[Candidate validation](https://github.com/writfold-dev/writfold/blob/4708ebfbdcaa087e6bc02540bb83ac46fd9b612a/crates/writfold-kb/src/gate/def_check.rs) rejects
repository scope on a candidate gate.

Consequently, naming a Vale config is not enough to bind every style rule,
dictionary, vocabulary file, imported config or selected executable. Editing
one of those resources must invalidate earlier applicability even when the
message is unchanged. This is source inspection, not an executed reproduction.

## Proof obligations

Proposed kernel: extend pure candidate pin/applicability decisions in
`crates/writfold-core`. Inputs are expected and observed payload/policy/tool
identities, declared root/selection identity, a canonical inventory with
completeness facts, and recorded input identities. Output is applicable,
stale, or unavailable/unknown with a reason.

- `candidate_inputs_current_iff`: current exactly when all required identities
  and complete declared inventories match the recorded ones.
- `incomplete_inventory_never_current`: unreadable, partial or missing required
  inputs cannot justify current evidence.
- `membership_change_stales`: adding, removing or renaming a selected path
  invalidates applicability, even if all surviving file bytes match.
- `policy_or_tool_change_stales`: changing selection policy, adapter, resource
  pin or selected executable/environment identity invalidates applicability.
- `irrelevant_change_preserves`: a path outside the declared selection does
  not affect identity, under the declared closure assumption.
- `canonical_inventory_order`: enumeration order and timestamps cannot change
  semantic identity; role/path/byte differences can.
- `historical_result_immutable`: later input changes alter applicability, not
  the recorded historical outcome.
- `mixed_snapshot_never_current`: an incomplete or inconsistent capture cannot
  authorize a current positive result.

Mutants omit a newly added rule, hash only existing paths, treat unreadable as
empty, drop a tool/profile binding, choose by timestamp, or accept an
incomplete capture. Reuse existing proved scope/inventory functions where their
laws already fit.

Tested and unproved: filesystem and Git identity, complete enumeration,
symlink/path resolution, snapshots, process invocation, executable verification
and rendering. A proof over declared inputs cannot establish that arbitrary
checker code reads no undeclared files or that a digest authenticates software.

## Behaviour

A gate declares named input files and resource roots, resolved explicitly
against the KB or subject repository, plus the selected tool/environment pin.
Status explains which dependency differs or cannot be read.

| Change or condition | Applicability |
|---|---|
| Same payload, policy, tool and complete declared inputs | Current |
| Edit only a rule, dictionary, config or adapter | Stale |
| Add, delete or rename a selected rule | Stale |
| Change selected tool or its verified environment identity | Stale |
| Missing required resource, unreadable root or partial enumeration | Unavailable/unknown |
| Change an explicitly excluded file | Unchanged under that declared selection |

Missing required inputs and a legitimate empty inventory are different states.
Whether a linter actually has applicable rules is a further check by its
adapter; a complete empty directory is not proof of useful coverage.

`deliver status`, resume, plan readiness and the delivery guard use the same
applicability decision. Reads do not execute tools, inspect remote services,
install dependencies or fetch packages. Returning exactly to the recorded
inputs may restore applicability under existing pin semantics; it does not
record a new execution.

## Design

Represent each input by a role, root identity, relative path and content digest.
Bind the selection itself, complete directory membership and required/optional
absence semantics. Define deterministic ordering and reject duplicate,
ambiguous or escaping paths. New inputs under a selected directory must be
seen; a manifest containing only yesterday's files is insufficient.

The initial recipe can declare the entire relevant Vale resource directory
rather than have Writfold parse Vale's configuration language. The adapter
checks that effective configuration and resources fall within that closure.
All effective vocabulary, imports, scripts, models or grammars, when used,
belong in it. Deliberately omitted sources must not silently fall back to
user-global resources.

Capture immutable input bytes and invoke the checker against that capture,
preserving the declared relative layout. Include the exact rendered payload.
Keep the historical report bound to the capture, and compare current source
identities before treating it as applicable. A changed source need not destroy
valid historical evidence; it prevents that evidence from standing for the
current delivery.

If the implementation cannot make a checker use captured inputs, do not claim
an exact snapshot from before/after hashes alone: a change-and-revert during
execution can evade them. Groom that limitation explicitly or refuse the
stronger contract. Writfold is not becoming an OS sandbox; admitted commands
remain within its cooperating-user trust model.

Pin and verify the executable selected, not just an executable name in PATH
or a self-reported version string. A tool/environment manifest may supply
expected identities, but the adapter must verify the actual executable and
effective resources used. Tool bytes needed for passive applicability checking
must be locally available; verification is not an installation operation.

Retain declared captures/reports through the existing receipt/artifact
mechanism. Reuse the current write protocol. If capture plus receipt adds a
publication boundary, extend `spec/KbWrite` and crash/fault tests; a partial
capture or dangling artifact reference cannot publish current passing evidence.
No new mutable authority database or report-import subsystem is introduced.

## Compatibility and reversibility

The owner's pre-alpha policy (#105/#107) permits simplifying definitions and
receipt schemas without compatibility readers or migrations. Unsupported old
evidence remains non-passing through the current invalid/unknown path; do not
delete user records or rewrite history.

Ordinary gates without additional checker inputs retain their intended current
workflow. This slice adds no mandatory Vale or Plyrune dependency.

## Alternatives

- Hash only the top-level config: misses transitive rules and resources.
- Bump a version manually whenever rules change: easy to forget and does not
  bind actual execution inputs.
- Re-run every checker on every status read: changes passive reads into
  execution and still leaves historical evidence underspecified.
- Parse each backend's config in Writfold: multiplies maintenance and couples
  policy semantics to individual tools.
- A general package manager or sandbox: exceeds the concrete input-binding need.

## Evidence and tests

Fixtures independently edit config, a nested rule, dictionary, adapter and
tool pin; add/delete/rename members; make a root unreadable; remove a required
resource; inject partial enumeration; and change input during capture and
execution. Include shared KB/project roots, symlink escape, identical bytes
under a different role/path, irrelevant excluded files, restored inputs and a
fresh clone missing the tool.

Test capture interruption and report publication if the write protocol changes.
Require kernel proof/mutant gates before wiring, then quality/unit checks.
The documentation-only proposal does not claim those checks or a benchmark
were run.

## Rollout

In Later: choose root/snapshot/tool identity representation; prove the
applicability laws; add local enumeration/capture and passive status checking;
exercise a generic synthetic checker; then integrate the Vale resource bundle.
Coordinate with #56/#63 and #106 without requiring their external tools.

## Unresolved questions

- Exact definition keys and how named roots compose with a separately stored KB?
- Which existing artifact path can retain captures for candidate gates?
- Snapshot layout and treatment of external absolute paths and symlinks?
- Minimal verifiable tool/environment identity without a new installer?

## Decision log

- 2026-11-24: Drafted for the owner's issues/RFC request. Implementation is
  outside this turn; no new authority store, checker or measured benefit exists
  as a result of this document.
