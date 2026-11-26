# Receiving work: a pre-receive hook and a required status check

> **Status:** built in v0.6.0-dev.25 ([#64](https://github.com/writfold-dev/writfold/issues/64)): the pre-receive hook recipe, tested by `crates/writfold/tests/receiver_hook.rs`. The GitHub required status check below is an example that is not exercised here. Signed receipts are not built.

[`handoff check`](gates.md#the-receivers-check-handoff-check) answers the receiver's question: did every gate I require pass, mechanically, on this candidate? It reads the required set and each gate's definition at the operator's policy revision, never at the candidate's. On its own it judges whichever run it is pointed at. A worker that does not cooperate can write that run's receipts. So the receiver runs the gates itself, into a KB only it writes, and checks that run.

Nothing a worker writes in its own run is read: not its receipts, plans or statements, and not the delivery modes or agent profiles it may have edited ([pinned agent closures](agents.md#pinned-with-the-runs-policy)).

## A pre-receive hook

`docs/guide/recipes/receiver/pre-receive` is a Git `pre-receive` hook for the bare repository. For each push to the protected ref (`WRITFOLD_PROTECTED`, default `refs/heads/main`), it:

1. takes the ref's current tip as the approved policy, never the pushed commit;
2. checks the pushed commit out into a temporary repository that borrows the bare repository's objects, quarantined ones included, and puts the policy revision's `.writfold/` over it;
3. runs every gate the policy's `.writfold/handoff.toml` requires, into a new run of the receiver's KB (`WRITFOLD_RECEIVER_KB`);
4. lands the push only when `writfold kb handoff check --policy <tip> --candidate <pushed>` accepts it, and otherwise refuses it with the check's rows.

Install it as `hooks/pre-receive`, with the variables set by a wrapper:

```sh
#!/bin/sh
export WRITFOLD_RECEIVER_KB=/srv/receiver-kb WRITFOLD_PROJECT=service
exec sh /srv/writfold/pre-receive
```

`docs/guide/recipes/receiver-rehearsal.sh` installs it on a bare repository and pushes each case:

| Push | Result |
|---|---|
| a candidate that passes the required gate | lands (`handoff=accepted`) |
| a failing candidate that weakens the gate's definition | refused: the policy's definition is run (`not-passed`) |
| a failing candidate that empties `required` | refused: the required set is the policy's |
| a failing candidate with a forged receipt and a plan that skips the gate | refused: neither is read |
| a push to another branch | not judged |

A push that creates the protected ref (there is no approved policy yet) or deletes it is refused.

## A required status check

`docs/guide/recipes/receiver/handoff-status.yml` does the same in GitHub Actions on a pull request, with the base commit as the policy. It is **not exercised here**. To rely on it:
- make its check required on the protected branch;
- set GitHub Actions as the check's expected source, so a status another app posts does not count;
- give the job no secrets, since it runs candidate code.

## What each prevents, and what it does not

Each prevents landing a commit on the protected branch whose required gates, as the policy defines them, do not pass on that commit when the receiver runs them. They do not prevent:
- **Bypass:** an administrator who pushes with hooks disabled, or who may bypass branch protection. For the status check, anyone who can change the workflow or the protection settings on the base branch.
- **Weak gates:** what a required gate checks is the policy author's. A test suite that misses a fault passes; the hook proves only that the policy's gates passed.
- **Undeclared inputs:** a gate's command may read files outside its `scope`, a network or a clock. The receipt binds the files in scope and the definition, not everything the command touched.
- **Candidate code:** the hook runs the gates, and so the candidate's code, on the receiver. Run it as a user with no credentials it does not need, on a machine where a candidate cannot reach the receiver KB's history.
- **Statements:** a gate that stands only on an attestation or a waiver is never accepted, whatever its `accept` says. A receiver that needs a person's approval asks for it outside this check.
- **Other workers' evidence:** the receiver trusts only runs it made. Evidence produced on another machine needs signing, which is not built ([#64](https://github.com/writfold-dev/writfold/issues/64)).

The receiver's KB is ordinary Git text: commit it to keep what each push was judged on.
