# Trial adversarial campaign, 9 November 2026

Milestone S4 of `docs/writfold/agent-boundary-plan.md`. `run.py` drives the
real `writfold trial` CLI and filesystem against a temporary repository; there
are no fault hooks in production code. `results.json` records the binary digest
and every case's expected and observed outcome.

| Case | Injection | Expected | Observed |
|---|---|---|---|
| preimage-race | worktree file changed between approve and apply | apply refuses, run abandoned | as expected |
| refuted-required | `approve --require safe` on a report refuting `safe` | approve refuses, no approval | as expected |
| unretained-cite | report cites a binding never retained | prepare refuses before creating a worktree | as expected |
| tampered-evidence | retained bytes altered after `retain` | prepare refuses | as expected |
| no-change | recipe whose candidate equals its source | prepare refuses | as expected |
| replay | apply twice, then recover | second apply and recover refuse; one outcome | as expected |
| kill-during-apply | SIGKILL `apply` after 50–600 ms, 60 runs | file matches the single recorded outcome | 24 killed after install → `recover` reconciled to applied; 24 killed before the claim → nothing written; 12 finished |
| checkout-untouched | after everything | user checkout byte-identical | as expected |

**Not covered here:**

- **An expired approval.** No clock control without root. The authorization
  unit tests cover each refused check on its own.
- **A kill landing between the claim and the install.** That window is a few
  milliseconds. The recovery unit tests construct that state directly, and
  recovery's decision is proved (`recovery_spec`).
- **An agent told to write the checkout directly.** The trial is cooperating,
  not a sandbox: nothing stops an agent's own shell. This campaign only shows
  that Writfold itself never writes the checkout.
