---
rfc: "2026-11-21-session-start-deadline"
title: "Session start: a deadline below the hook timeout"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/69
milestone: "v0.6.0-dev.15"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Session start: a deadline below the hook timeout

## Proof obligations

None: no decision logic. Which parts are injected past the deadline is a
fixed list, and the deadline is a timer (IO), tested by
`crates/writfold/tests/kb_hook.rs` with a zero deadline.

## Decision

Session start runs the brief on a thread and waits at most 20 s
(`WRITFOLD_HOOK_DEADLINE_MS`), below the plugin's 30 s timeout. Past it, the
hook injects the parts that need no index (behind-remote, inbox) and
`index not built yet: run writfold kb index rebuild, then writfold kb
brief`, then exits; the build is abandoned. A build publishes by rename
(`spec/KbRepair.tla`), so nothing half-built is published, and the
temporary file is swept by a later build.

Whether a build may continue past the hook was an owner question; the
default, accepted on 21 November (#73), is a notice only: AGENTS.md rules
out a companion process, so no detached build is started.

## Decision log

- 2026-11-21: accepted (notice only) under the owner's standing acceptance;
  implemented for v0.6.0-dev.15.
