# Governance

Writfold has one maintainer, the owner. The owner decides what is accepted,
what ships and what is frozen. The standing decisions are recorded in the
[mission](docs/writfold/mission.md) and in [AGENTS.md](AGENTS.md).

## Who does what

- **The owner** accepts or declines issues, accepts or rejects
  [RFCs](docs/rfcs/README.md), sets milestones, and is the acceptance test:
  Writfold is done when the owner uses it for real work.
- **Agents** (coding sessions working for the owner) may file issues, draft
  RFCs, implement accepted work, run the gates, cut pre-releases they have
  been cleared to cut, and close the issues their commits resolve. Planning
  agents explore and write plans only. An agent never accepts its own RFC.
- **Contributors** open issues. A pull request needs an `accepted` issue or
  RFC behind it, unless it is a trivial fix ([CONTRIBUTING.md](CONTRIBUTING.md)).

## Where decisions live

| Kind | Where |
|---|---|
| Direction and scope | [mission.md](docs/writfold/mission.md) |
| Design of a change | an RFC in [docs/rfcs/](docs/rfcs/README.md) |
| Work to do, with its history | GitHub issues, grouped into milestones |
| What changed for users | [changes.md](docs/guide/changes.md) and the release notes |
| What was built, as built | [kb-v1-design.md](docs/writfold/kb-v1-design.md) |
| Evidence that it works | gate receipts named in commit messages, and the proofs gate |

## Milestones

One milestone per planned release (for example `v0.6.0-dev.11`), plus:
- `v0.6.0`, the work that must land before the stable release;
- `Later`, accepted work with no release planned yet.

An issue with no milestone has not been triaged for a release. A milestone
closes when its release is tagged. Anything still open in it moves to the
next milestone, and the move is recorded in a comment.

## Labels

- **Triage:** `needs-triage`, `accepted`, `blocked`, `needs-owner` (a
  question only the owner can answer), `declined`.
- **Design:** `rfc` (the issue tracks an RFC), `needs-rfc`, `needs-proof`
  (decision logic: a Lean proof is due before wiring).
- **Size:** `size/S`, `size/M`, `size/L`.
- **Area:** `area/stores`, `area/gates`, `area/runs`, `area/find`,
  `area/watch`, `area/proofs`, `area/docs`, `area/release`.
- **Type:** `bug`, `enhancement`, `documentation`.

## Frozen components

Tuft, Weft and Splicery are frozen (owner, 15 November 2026). Issues against
them are accepted only when a change elsewhere breaks them.
