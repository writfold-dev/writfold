# Writfold

**Deterministic agent work that resumes where it stopped.**

Writfold keeps a coding agent's plan, checks and decisions as plain files in
Git. What is done, what is still owed and whether a check still holds are
computed from those files, the same on every machine, so a fresh session on
any machine reads a short brief and carries on.

> **Pre-release (`v0.6.0-dev.30`).** Built in public, for one person and their
> agents; formats and commands may still change. Linux x86_64 or WSL2.

No server, no account, no model: Writfold writes files and reports what they
say. Every answer names the file and line it came from and says when that
source has moved.

## Install

```sh
curl -fsSL https://writfold.dev/install.sh | sh
```

Or download [the static binary](https://github.com/writfold-dev/writfold/releases/download/v0.6.0-dev.30/writfold-x86_64-linux-musl)
and its checksum by hand; [Installing](docs/guide/install.md) has the steps,
the Claude Code plugin and building from source.

## In one minute

```console
$ writfold kb checkpoint --new-run --project demo-api --task "batch ledger writes" \
    --note "Next: tune the batch size."
$ writfold kb gate run unit          # a check whose receipt is tied to the bytes it checked
$ git add -A && git commit -qm "checkpoint" && git push -q
```

After someone edits a file the gate covers, on another machine:

```console
$ writfold kb resume
gate unit [stale] … !stale
  why: changed in scope: src/ledger.rs
  next: writfold kb gate run unit
checkpoint … note: Next: tune the batch size.
```

## Learn more

- [A tour](docs/guide/tour.md): sourced answers, runs and plans, gates,
  your policy, delivery checks, guard rules, `writfold watch`, and what works
  today.
- [The guides](docs/guide/README.md), one page per area.
- [What is proved](docs/guide/assurance.md): the decisions are proved in Lean,
  the protocols model-checked in TLA+, and the limits.
- [Mission](docs/writfold/mission.md), [design](docs/writfold/kb-v1-design.md)
  and [RFCs](docs/rfcs/README.md).

## Contributing

Work is tracked in [issues](https://github.com/writfold-dev/writfold/issues)
and [milestones](https://github.com/writfold-dev/writfold/milestones).
[CONTRIBUTING.md](CONTRIBUTING.md) says where to start and maps the
repository; [GOVERNANCE.md](GOVERNANCE.md) says who decides.

## Licence

Writfold and its companion website use the [MIT license](LICENSE), selected by Tarek Nawara on 2 November 2026. Third-party materials retain their original notices and licenses; this does not relicense bundled reference material or authorize publication of private playbook content.
