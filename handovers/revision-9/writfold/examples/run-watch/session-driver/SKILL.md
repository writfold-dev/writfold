---
name: writfold-session-driver
description: Start or recover a tracked Writfold change, hand the user its watch command, or resume an existing run under its pinned procedure. Use on the configured WSL host for explicit tracking or a tracked change procedure.
---

# Writfold session driver

Candidate package source for the Writfold host. Activate this revision only after normal package admission. Use [the run interface](references/run-interface.md) when a branch below needs detail.

1. Classify the request. “What's the ID?”, “show progress” and “open the TUI” are reads. Resolve the durable task binding or a supplied RunRef. If none exists, say “This work has no tracked run yet.” If several fit, show the permitted candidates and ask which. Do not start tracking on a read request.
2. For a tracked change, bind the authorized realm, objective, completion boundary, repositories, procedure revision, baseline context and authority reference. Use the host's validated intake codec and persist one request key for this logical task before calling start. Use the installed, qualified Writfold interface. Missing host access, unsupported capability or missing grant is a blocker.
3. Submit the structured request. On an uncertain response, recover the same key and replay identical input. On conflict or inability to recover it, stop and surface the conflict. Use only the RunRef in the committed acknowledgement. Bind it to the task. Never generate an ID or guess from a branch, title or global last run.
4. Hand the user the ID and a standalone, copyable `writfold run watch <RunRef>` command immediately. Name the completion boundary in one sentence. Continue their authorized task. Keep stdout/JSON for the host; send the human the short receipt.
5. For status, read a single snapshot. Reply with the RunRef, cut, observed blocker or achieved boundary and next evidence. For exact replay give `writfold run show <RunRef> --at <event>`. Reuse the run across worktrees and chats.
6. For continuation, reload the pinned skill/procedure closure. Check the authoritative head, ownership, grant, repo bindings and pending effects before any mutation. If another writer owns it, remain a reader until the coordinator grants a valid transfer. Reconcile unknown effects under their original EffectIds.
7. Pass the bound RunRef into the existing author-change procedure before worktree checkout. Its controlled adapters record worktrees, tests, review, docs and PR receipts. Chat narration is an observation; show a pass only from qualified evidence. Closing the viewer or chat has no authority over the run. Record a terminal result only against the requested finish line; report unresolved effects separately. Learning remains candidate evidence.

If you cannot reach the configured host, say “I can’t see the Writfold ledger from this session, so I can’t give you a verified run ID.” A demo ID belongs only in a labelled example.
