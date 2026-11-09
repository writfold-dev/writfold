# Trial profile: prepare, approve, apply

`writfold-trial-worktree/1` is the first connected path through Writfold's edit
flow. It is a **cooperating single-user trial**, chosen by the owner on
8 November 2026 so people can try the flow on ordinary repositories. It is not
the hardened alpha worktree path (pinned Worktrunk, loose-object repositories
only), and it does not change that path's requirements.

## Try it

```sh
cargo build --release -p writfold
export WRITFOLD_STATE=/tmp/writfold-trial      # any new directory
target/release/writfold trial init
target/release/writfold trial prepare --repo /path/to/your/repo \
    --file src/client.rs --recipe add-retry.spl
target/release/writfold trial approve <run> [--require <weft-check>]...
target/release/writfold trial apply <run>
target/release/writfold trial show <run>
target/release/writfold trial recover <run>   # only after an interrupted apply
git -C "$WRITFOLD_STATE/worktrees/<run>" diff
```

Recipes are [Splicery](https://github.com/writfold-dev/splicery) recipes; see its
`examples/try/` for one that works on a small Rust file.

## What each step does

| Step | Effect | Ledger record |
|---|---|---|
| `init` | Creates the private state directory: an Ed25519 operator key, a bootstrap policy trusting it, the Git executable's observed digest, the ledger repository, the object store | — |
| `prepare` | Adds a detached, **sparse** worktree of your repository's **HEAD commit** inside the state directory (only the edited file is checked out; everything else is skip-worktree, so `git status`/`diff`/commits behave normally and cost does not grow with repository size), replays the recipe through the pinned Splicery kernel, builds and validates the exact `writfold-worktree/1` prepared edit, retains source, recipe, candidate and report. Refuses a candidate identical to its source, and a `--report` that is not a complete `weft.report/1` claiming no grounding or effect authority | `Prepared` |
| `approve` | With `--require CHECK` (repeatable), first reads the retained Weft report and refuses unless every named check is present and `entailed` (the proved `required_entailed`). Then signs an operator grant (Write on this run's worktree, one hour) and a prepared-edit approval binding the prepared-edit digest, report, procedure, resource generation and validity | `Approved` |
| `apply` | Verifies both signatures strictly under the policy, every bound value and the validity window, and asks the kernel for a current grant covering Write; records a durable claim; checks the worktree file still holds the retained source bytes; installs the candidate through the staged installer | `Claimed`, then `Applied`, or `Abandoned` if the original was never replaced |
| `recover` | For a claim with no outcome (a crash or error during install): observes the file without rewriting it. The candidate's exact bytes mean it was installed; the source's exact bytes mean it was not; anything else stays Unknown and refuses | `Applied` (reconciled) or `Abandoned` |
| `show` | Replays the run's ledger history through the kernel and prints it with five independent badges: evidence, assessment, approval, claim and effect. There is no aggregate status | — |

Every append goes through the [durable local ledger](semantic-core.md#durable-local-ledger):
the run's history is replayed through the kernel and the edit-flow validator
(exactly `Prepared` → `Approved` → `Claimed` → `Applied` or `Abandoned`, each
carrying its material, with the claim and `Applied` naming the prepared
candidate and an abandonment saying why) before the head moves.

## What it does not do

- It never writes to your checkout. Uncommitted changes are not included: the
  worktree starts at HEAD. Bring the change back with `git diff` or by
  committing in the worktree.
- It adds worktree metadata to your repository (`git worktree list` shows it).
  A refused `prepare` removes the worktree it created.
- It assumes nobody races it on the worktree and that the repository is not
  hostile; it is not a sandbox. The operator key lives in the state directory.
- A Weft report is optional. Without `--report`, an empty report is retained
  and bound, and `--require` refuses. With one, Weft's results are recorded as
  Weft states them, local and **hypothetical**, never as grounded facts. The
  required check names are enforced at `approve` but not yet carried inside the
  signed approval.
- Recovery's decision and the required-check gate are proved over the shipped
  Rust (`docs/writfold/aeneas.md`); the rest of the flow is tested.
- A claim is never re-dispatched: after an interrupted `apply`, `apply`
  refuses and `recover` decides from the file's exact bytes. If the file holds
  neither the source nor the candidate, the outcome stays Unknown for a person
  to resolve. A leftover staging file from an interrupted install is not
  cleaned up yet.
- The procedure identity is a fixed digest of the profile name, not an
  admitted procedure. Grants are not stored durably beyond their retained
  envelopes, and there is no revocation command yet.

## Evidence

Tests cover the full flow on a real repository, every out-of-order step, a file
changed after approval, an approval under a different policy, an ambiguous
recipe and escaping paths (both leave no worktree), and each authorization
check alone: an untrusted grant signer, an approval for a different prepared
edit, and a grant that does not cover the worktree. Removing the grant
verification, the approval verification, the current-grant check, the file
guard or the validator's material rule each fails a test. Recovery is tested
for all three observations (installed, not installed, neither), plus a claim
that `apply` must not re-dispatch; removing either recovery comparison, the
claim's candidate binding or the abandonment-reason rule fails a test. Two
checks are defensive only: the explicit "claimed without an outcome" message in
`apply` (the history-shape match already refuses) and the outcome rule that the
third record is the claim (the ledger re-validates the prefix, so nothing else
can be third).
