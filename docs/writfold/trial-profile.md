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
target/release/writfold trial approve <run>
target/release/writfold trial apply <run>
target/release/writfold trial show <run>
git -C "$WRITFOLD_STATE/worktrees/<run>" diff
```

Recipes are [Splicery](https://github.com/writfold-dev/splicery) recipes; see its
`examples/try/` for one that works on a small Rust file.

## What each step does

| Step | Effect | Ledger record |
|---|---|---|
| `init` | Creates the private state directory: an Ed25519 operator key, a bootstrap policy trusting it, the Git executable's observed digest, the ledger repository, the object store | — |
| `prepare` | Adds a detached worktree of your repository's **HEAD commit** inside the state directory, replays the recipe through the pinned Splicery kernel, builds and validates the exact `writfold-worktree/1` prepared edit, retains source, recipe, candidate and report | `Prepared` |
| `approve` | Signs an operator grant (Write on this run's worktree, one hour) and a prepared-edit approval binding the prepared-edit digest, report, procedure, resource generation and validity | `Approved` |
| `apply` | Verifies both signatures strictly under the policy, every bound value and the validity window, asks the kernel for a current grant covering Write, checks the worktree file still holds the retained source bytes, then installs the candidate through the staged installer | `Applied` |
| `show` | Replays the run's ledger history through the kernel and prints it | — |

Every append goes through the [durable local ledger](semantic-core.md#durable-local-ledger):
the run's history is replayed through the kernel and the edit-flow validator
(exactly `Prepared` → `Approved` → `Applied`, each carrying its material, and
`Applied` installing the prepared candidate) before the head moves.

## What it does not do

- It never writes to your checkout. Uncommitted changes are not included: the
  worktree starts at HEAD. Bring the change back with `git diff` or by
  committing in the worktree.
- It adds worktree metadata to your repository (`git worktree list` shows it).
  A refused `prepare` removes the worktree it created.
- It assumes nobody races it on the worktree and that the repository is not
  hostile; it is not a sandbox. The operator key lives in the state directory.
- No Weft report is required. Without `--report`, an empty report is retained
  and bound, so the approval says only that no report was supplied.
- If the process dies after installing but before `Applied` is recorded, the
  outcome is Unknown: a later `apply` refuses because the file no longer holds
  the source bytes, and the worktree shows what happened.
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
guard or the validator's material rule each fails a test.
