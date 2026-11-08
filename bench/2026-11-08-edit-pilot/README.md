# Edit pilot, 8 November 2026

A small **debug pilot** following [the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md).
It is not a headline result: one repeat per cell, six tasks, one model.

## Question

On precise source edits, does making changes through Splicery (arm B) reduce
mistakes or tokens compared with an agent's ordinary file tools (arm A)?

The connected Writfold flow (retained evidence, approval, apply in an owned
worktree) does not run yet, so this pilot cannot measure it. It measures the
Splicery edit step alone.

## Method

- Fixture: a small Rust/config repository in [`fixture/`](fixture/).
- Six tasks in [`tasks.json`](tasks.json), each aimed at a common edit mistake:
  a duplicate match (T1), a long expression to preserve (T2), five counted
  lines next to a commented decoy (T3), an insert inside a window when the same
  line also exists in a test module (T4), a value swap (T5), and an ambiguous
  instruction where the correct outcome is to change nothing and say why (T6).
- Every task has a reference Splicery recipe ([`reference-recipes/`](reference-recipes/))
  that produces the hidden expected bytes ([`expected/`](expected/)), so each
  task is solvable in both arms.
- Each trial ran as a fresh Claude Code subagent on Sonnet in its own copy of the
  fixture, with the exact prompt in [`prompts/`](prompts/). Arm B could read
  Splicery's public language reference and examples; that reading counts.
- [`grade.py`](grade.py) compares every file byte for byte with the expected
  tree and flags any other changed or added file. Agents never saw the expected
  files or the grader. T6 also requires a refusal reply.

## Results

| | Arm A: ordinary tools | Arm B: Splicery |
|---|---|---|
| Correct, exact bytes, nothing else touched | 6 / 6 | 6 / 6 |
| Out-of-scope changes | 0 | 0 |
| Correct refusal on the ambiguous task | yes | yes |
| Total tokens (6 trials) | 208,074 | 234,106 |
| Tool calls | 24 | 30 |
| Wall time | 51.9 s | 79.3 s |

Per-trial data is in [`results.json`](results.json); the recipes arm B agents
wrote are in [`agent-recipes/`](agent-recipes/).

## Reading

- **No precision difference.** Both arms were perfect. These single-file tasks
  are within what an agent's ordinary edit tool already does safely: it refuses
  a non-unique match, so the duplicate-match traps did not trigger mistakes.
- **Splicery cost more.** About 12.5% more tokens, 25% more tool calls and 53%
  more wall time, mostly one extra step to write and preview a recipe.
- **Recipes came easily.** Arm B agents wrote valid recipes first time, including
  counted matches and window-scoped inserts. One (T2) used raw byte offsets,
  which worked but would break on any earlier change.
- **Found a footgun while building controls:** `after <line capture> insert lines`
  inserts before that line's terminator, joining two lines. The reference
  recipe uses `before <next line>` instead.

## Limits

One repeat per cell and six small tasks give no statistical power; differences
of this size could be noise. Token totals are Claude Code's per-subagent usage
reports and include each subagent's fixed host context, not raw provider usage
events. The model alias does not expose an exact backend revision.

## Next

The plan's hypotheses for a measurable win are elsewhere: larger files with many
near-duplicate sites, long preserved expressions, edits that must survive
concurrent changes, and multi-session work where retained evidence avoids
rebuilding context. Those need the connected Writfold flow and harder fixtures.

Reproduce: copy `fixture/` per trial, give the agent the prompt with `<trial>`
and `<splicery-cli>` filled in, then run
`python3 grade.py fixture . <trials-dir>` with one directory per trial named
`T<n>-<arm>` containing `repo/` and `reply.txt`.
