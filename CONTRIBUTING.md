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

Read [AGENTS.md](AGENTS.md) first. In short:

- Build through Compose/BuildKit. The release target is a static musl
  executable.
- Before pushing `main`, run `writfold kb gate run quality` and
  `writfold kb gate run unit` (or `scripts/gate quality|unit`) from `main`,
  and put the receipt lines (`writfold kb gate lines`) in the commit message.
- If you change `crates/writfold-core` or `proofs/`, run the proofs gate:
  `docker compose -f compose.proofs.yaml run --build --rm proofs`.
- Decision logic is proved in Lean before it is wired into a verb
  ([aeneas.md](docs/writfold/aeneas.md)).
- Respect the size budgets. No `unwrap` or `expect`.

## Commits

- Name the issue: `Refs #N` while work is under way, `Closes #N` on the
  commit that finishes it.
- If users can notice the change, add a line to
  [changes.md](docs/guide/changes.md) and update the guide page it touches,
  in the same commit.
- If the work belongs to an RFC, update its `implementation` field and its
  registry row.
