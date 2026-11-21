---
rfc: "2026-11-21-record-option-ownership"
title: "Record options belong to the kind that consumes them"
status: accepted
implementation: complete
authors: [astra]
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/85
milestone: v0.6.0-dev.15
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Record options belong to the kind that consumes them

## Proof obligations

None: this changes which command lines parse, not a product decision over a typed request. Clap parsing, help and typed conversion are tested. The write planners still decide whether assets exist and whether a write is admissible; proposal encoding, digests, replay and on-disk formats do not change.

## Evidence

On the released `v0.6.0-dev.13` binary in a scratch Git KB:

```text
writfold kb record investigation --title 'Temperature again' --prs PR-42
```

Exit 0; the new investigation contained neither `prs` nor `PR-42`. A ticket accepts `--prs`; an investigation never consumes it. This is worse than duplicated strings: a caller sees success and loses supplied information. [The pasted tables](../../crates/writfold-app/src/kb/investigation.rs) are still live after [#55](https://github.com/writfold-dev/writfold/issues/55). [Clap construction](../../crates/writfold-app/src/kb/cli.rs) takes the top-level verb's union; [record construction](../../crates/writfold-app/src/kb/writes.rs) dispatches by positional kind later. `propose` shares that constructor but adds its own scope and storage flags. `--project` travels in `Common` rather than the ordinary value map.

## Behaviour and design

Use one **compiled** set of options per `ticket`, `decision`, `link`, `investigation`; compose the common KB flags and, only for `propose`, `--session`, `--run`, `--tracked`, `--local`. Use the same kind definition for parsing and help, under both verbs. Remove the two handwritten unions and the independent synopsis of kind options; do not add an external schema file or a second parser. Keep the typed write models and stable proposal codec. Help must work without a KB.

`record investigation … --prs PR-42` and its `propose` counterpart become usage errors (exit 1, name the option and kind, no file/proposal). `record investigation --help` shows `--question`, not `--prs`; `propose ticket --help` shows `--prs` and proposal scope. Preserve `--project` on a decision and the two meanings of `--signatures` (list on investigation; switch on `find`). Existing shared flags, supported option positions including before the kind, and valid calls keep their meaning. For options given before the kind, bind them to the selected kind as well: there must be no bypass. No new CLI aliases or dynamic flags. Project data belongs in the [asset catalog](2026-11-21-project-asset-catalogs.md), not in the option parser.

## Alternatives, compatibility and rollout

Just deriving `PROPOSE = RECORD + extras` still accepts the wrong kind's flag. Moving the strings to `.writfold/` would give the same command different meanings in different clones and cannot teach an old binary how to persist a new field. Silently ignoring the flag for compatibility is exactly the bug. Only invocations supplying fields that the kind discarded must change; remove that option or choose the kind that records it. No migration for old proposals.

[P1, #85](https://github.com/writfold-dev/writfold/issues/85), dev.15; ready independently of catalog proofs. Exercise the public CLI for all four kinds, shared options on either side of the kind, wrong-kind field in direct/local/tracked paths (no mutation), scoped help, and valid direct/propose/accept calls with preserved fields. Keep existing stored proposal IDs readable. Update `docs/guide/changes.md` when shipped. Quality/unit receipts on main before closing.

## Decision log

2026-11-21 — Accepted under the owner's standing default. Fix silently dropped input, not the product's record schema. No unresolved acceptance questions.

2026-11-21 — Implemented for v0.6.0-dev.15: `crates/writfold-app/src/kb/kinds.rs` is the one table (the verb-wide lists are its union; the usage synopsis is checked against it by a unit test); `crates/writfold/tests/kb_record_options.rs` exercises the four kinds, an option before the kind, direct, `--local` and `--tracked` paths with the KB and both queues untouched, valid direct and proposed writes, and scoped help.
