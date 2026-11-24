---
rfc: "2026-11-24-delivery-check-diagnostics"
title: "Retain structured diagnostics from delivery checks"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/115"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Retain structured diagnostics from delivery checks

## Summary

Extend candidate command gates with a structured report that retains advisory
findings even when the check passes. Keep execution, report completeness,
policy violations and presentation separate. Reuse Writfold's existing
receipts and gate outcomes.

This is one design slice of [#119](https://github.com/writfold-dev/writfold/issues/119).
The semantic contract below is proposed; field names and wire version are
for grooming and are not shipped configuration.

## Motivation

At `4708ebfbdcaa`, [script evaluation](https://github.com/writfold-dev/writfold/blob/4708ebfbdcaa087e6bc02540bb83ac46fd9b612a/crates/writfold-kb/src/deliver/script.rs)
turns exit 0 into a pass with an empty findings list. On nonzero exit, each
stdout line becomes a finding with the gate's ID; line and text survive, but
backend rule identity and severity do not. Ordinary nonzero exits all fail,
whereas signals, timeouts and spawn errors are unknown.

That is sufficient for a simple commit-format script, but a useful advisory
linter needs to say both "the check completed" and "here is advice". Source
inspection establishes this behavior; no Vale runtime reproduction or quality
measurement is claimed.

## Proof obligations

Proposed kernel: extend the existing pure gate/delivery decision modules in
`crates/writfold-core` with report admission and classification. Inputs are
execution ending, supported/valid report state, assessment completeness,
declared outcome, diagnostic severities/rule IDs and the bound blocking
policy; output is Pass, Fail or Unknown with a reason. IO supplies these facts.

- `report_pass_iff`: pass exactly when execution and report agree on a
  completed assessment and the declared policy has no blocking finding.
- `incomplete_report_never_pass`: missing, unsupported, malformed or incomplete
  reports cannot satisfy the gate.
- `execution_failure_never_pass`: timeout, signal, spawn failure or a
  conflicting process/report outcome cannot become a pass.
- `advisory_preserves_pass`: adding only nonblocking findings to an otherwise
  passing complete assessment preserves pass and retains those findings.
- `blocking_finding_prevents_pass`: a finding selected by blocking policy
  cannot be hidden by lower-severity findings.
- `presentation_independent`: sorting, display limits and formatting do not
  affect classification.
- `unknown_not_waived_implicitly`: the report path never manufactures a waiver
  or attestation; existing gate admission remains authoritative.

Mutants accept partial reports, drop blocking findings, erase warnings on
pass, use only the displayed prefix to decide, ignore a process failure, or
promote unknown to pass. Applicable existing pin laws cover changed report
policy and payload; input closure is the companion RFC.

Tested and unproved: decoding, JSON size/depth limits, process supervision,
location validation, report retention, file IO and text/JSON rendering.
Proofs establish policy application, not the correctness of a linter's advice.

## Behaviour

An illustrative retained finding has:

```json
{
  "rule": "House.Terms",
  "severity": "warning",
  "location": {"line": 3, "column": 8, "end_column": 15},
  "message": "Prefer 'use' here.",
  "link": null
}
```

Location is relative to the exact delivered payload; whole-document findings
have an explicit absent location. Coordinates and encoding must be specified
by the wire contract and normalized by adapters, not guessed by the renderer.

| Assessment | Result under the declared policy |
|---|---|
| Completed, valid report; no blocking findings | Pass; advisory findings retained and shown |
| Completed, valid report; blocking findings | Fail; violation diagnostics retained |
| Checker unavailable, crashed or timed out | Unknown; operational reason shown |
| Missing, malformed, unsupported or incomplete report | Unknown; never a clean report |
| Exit/report disagreement or changed checked inputs | Unknown/non-current; cannot satisfy delivery |

For the structured path, propose exit 0 for a completed nonblocking result,
exit 1 for a completed blocking result, and exit 2 for incomplete assessment.
The report declares the same outcome and completeness. All other exits or
disagreement are unknown. The adapter translates the backend's own exit
conventions; Writfold never infers backend semantics from a numeric exit alone.

A successful advisory check remains ready only if the mode's other obligations
are met. `deliver check`, `deliver status` and JSON read the same findings.
Stale findings remain historical and are not drawn against edited text.
A plain text command-gate path may remain an explicit current mode.

## Design

Use one bounded structured object per check, containing outcome, completeness,
diagnostics and optional suppression/coverage information. Bind the transport
profile and blocking policy into the gate definition. Reject invalid or
contradictory records rather than partially reading them as a pass.

Retain the full supported report within the existing receipt or as an immutable,
digest-bound artifact it references. The parser must consume the complete
object before deciding. A transport limit that prevents this makes assessment
unknown. A display limit may abbreviate presentation but points to retained
evidence and leaves the verdict unchanged. Do not create a second receipt
ledger or interpret a content digest as authentication.

Preserve backend rule identity and severity; retain an explanation link when
provided. Do not persist matched prose merely to reproduce a diagnostic if
location plus message is sufficient. Bound all text, counts and positions.
Any optional actionable replacement remains a suggestion, never an automatic
edit made by the checking path.

Reuse the existing receipt publication protocol. If artifact publication adds
a new multi-file boundary, extend `spec/KbWrite` and fault tests so interruption
cannot publish a receipt whose required report is missing. Rendering remains
tested IO; no model call occurs on a read.

## Compatibility and reversibility

Under the owner's pre-alpha direction (#105/#107), current report/receipt
formats may be simplified without migration readers or compatibility shims.
Unsupported evidence is invalid/unknown, never a current pass. Keep intentional
current plain-text gates if useful; that is distinct from maintaining retired
schema fallbacks. Do not delete user records or rewrite history.

No Vale dependency is introduced by this slice. A KB without a structured
checker retains its ordinary workflow.

## Alternatives

- Print warnings only during execution: they disappear on resume and cannot
  explain a later decision.
- Return failure for every warning: defeats an advisory rollout.
- Parse Vale directly in Writfold's kernel: couples generic evidence semantics
  to one backend.
- Add a universal checker SDK or second ledger: unnecessary for one concrete
  report contract and the existing command-gate mechanism.

## Evidence and tests

Exercise pass with zero findings, pass with warnings, blocking findings,
operational failure, malformed/missing/oversized reports, conflicting exits,
invalid locations, Unicode positions, display truncation and stale payloads.
Read retained results after a fresh process and compare text/JSON semantics.
Test that changing only blocking policy or report profile makes prior evidence
non-current. Preserve existing waiver, readiness and guard behavior.

Implementation requires the proof gate before decision wiring, then the
ordinary quality/unit gates. Documentation-only drafting does not claim these
gates have run.

## Rollout

In Later: agree the bounded transport and outcome mapping; prove the pure
classification laws; add capture/retention and rendering; exercise with a
synthetic command checker. The optional Vale recipe consumes this facility
after the input-binding slice; no other backend is required to ship it.

## Unresolved questions

- Exact schema name, diagnostic count/size caps and location convention?
- Embed the bounded report in the receipt or retain one referenced artifact?
- Which current text-only gate diagnostics should share the same renderer?

## Decision log

- 2026-11-24: Drafted for the owner's requested issues/RFCs. This turn is
  planning only; no implementation or benchmark run was requested. The usual
  default-acceptance process can apply when implementation is taken up.
