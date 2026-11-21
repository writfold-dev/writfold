# Contributing

The rules and who decides are in [GOVERNANCE.md](GOVERNANCE.md). This page is
the how-to.

## Start in the right place

| I want to… | Go to |
|---|---|
| Report something that is broken or wrong, or a regression | a [bug report](../../issues/new?template=bug_report.yml) |
| Propose a new capability or a change in behaviour | a [feature or design proposal](../../issues/new?template=feature_request.yml) |
| Implement something | a pull request that links an `accepted` issue or an accepted [RFC](docs/rfcs/README.md) |

Issues are the only intake channel. New issues get `needs-triage`. The owner
accepts them, labels them `needs-rfc` when the design must be written down
first ([when an RFC is required](docs/rfcs/README.md#when-an-rfc-is-required)),
and puts them in a milestone.

A trivial fix can go straight to a pull request without an issue: a typo, a
doc correction, a dependency bump, or an obvious one-line fix with no change
in behaviour. If you cannot tell whether it is trivial, it is not. Open the
issue.

## Building and checking

Read [AGENTS.md](AGENTS.md) first. Two kinds of check apply, and neither
stands in for the other.

### Checks you can run from a fresh clone

With the toolchain that `rust-toolchain.toml` pins (Rust 1.98.0) and
Python 3.11 or newer; the first two were exercised from a fresh clone on 21
November 2026:

```sh
export RUSTUP_TOOLCHAIN=1.98.0   # the quality script also builds crates outside the checkout
CARGO_TARGET_DIR=$PWD/target python3 scripts/writfold_quality.py --evidence /tmp/writfold-quality
cargo test --workspace --locked
```

If you changed `crates/writfold-core` or `proofs/`, also run the proofs gate
(Docker; it needs no private material, and a full run with its mutant
calibration takes hours):

```sh
docker compose -f compose.proofs.yaml run --build --rm proofs
```

The first runs rustfmt, clippy (`-D warnings`), the size budgets and the
architecture checks of `crates/writfold-xtask`, with their calibrations;
it exits 0 only when every check passes and writes its findings under
`--evidence`. The proofs gate regenerates the Lean translation, refuses
drift, builds the proofs, audits their axioms and calibrates the mutants.
Decision logic is proved in Lean before it is wired into a verb
([aeneas.md](docs/writfold/aeneas.md)). No `unwrap` or `expect`.

### Release gates (owner, on main)

Before pushing `main`, the owner runs `scripts/gate quality` and
`scripts/gate unit` (or `writfold kb gate run quality|unit`) from `main`
and records the receipt lines in the commit message. These wrappers run the
same checks in an admitted tool image (`scripts/gate bootstrap`) with the
operator's storage and a pinned private playbook, so they cannot be run
from a public clone; a pull request's checks above are not a substitute
for them, and they are not a substitute for a reviewer reading the change.
This repository has no public CI at present.

## Commits

- Name the issue: `Refs #N` while work is under way, `Closes #N` on the
  commit that finishes it.
- If users can notice the change, add a line to
  [changes.md](docs/guide/changes.md) and update the guide page it touches,
  in the same commit.
- If the work belongs to an RFC, update its `implementation` field and its
  registry row.
