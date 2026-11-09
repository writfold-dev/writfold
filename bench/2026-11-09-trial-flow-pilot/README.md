# Trial-flow pilot, 9 November 2026

The second **debug pilot** under [the benchmark plan](../../docs/writfold/agent-integration-benchmarks.md).
One repeat per cell, six tasks, one model: not a headline result.

## Question

What does routing an agent's edit through the connected Writfold flow
(`writfold trial prepare → approve → apply`) cost, compared with an agent
editing the checkout with its ordinary tools, and do Writfold's guarantees hold
when an agent drives it?

## Method

- Same six tasks, fixture and hidden expected bytes as the
  [8 November edit pilot](../2026-11-08-edit-pilot/), with the fixture committed
  to a Git repository per trial.
- **Arm A:** ordinary file tools editing the checkout.
- **Arm W:** the agent writes a Splicery recipe and runs `prepare`, `approve`
  and `apply`; it must not modify the checkout. In real use `approve` is the
  operator's step; here the agent ran it too. `writfold trial init` ran in the
  harness, outside the measured cost.
- Each trial was a fresh Claude Code subagent on Sonnet with the prompt in
  [`prompts/`](prompts/). [`grade.py`](grade.py) checks exact bytes: arm A's
  checkout must equal the expected tree; arm W's checkout must be byte-identical
  to the fixture, the run's worktree must hold the expected bytes, and
  `writfold trial show` must validate a history ending in `applied`. For the
  ambiguous task the right outcome is no applied run in either arm.

## Results

| | Arm A: ordinary tools | Arm W: Writfold trial flow |
|---|---|---|
| Correct, exact bytes | 6 / 6 | 6 / 6 |
| Correct refusal of the ambiguous edit | yes | yes |
| Checkout left untouched | n/a (edits the checkout) | 6 / 6 |
| Applied edits with signed approval, claim and outcome in a validated ledger | n/a | 5 / 5 |
| Mistaken preparation caught before approval | n/a | 1 |
| Total tokens | 203,843 | 235,976 (+15.8%) |
| Tool calls | 24 | 35 |
| Wall time | 99.4 s | 145.3 s |

Per-trial data, including each Writfold run's validated ledger history, is in
[`results.json`](results.json); the recipes arm W agents wrote are in
[`agent-recipes/`](agent-recipes/).

## Reading

- **Correctness was equal.** Both arms were perfect, as in the first pilot.
- **The governed flow costs more:** about 16% more tokens, 11 more tool calls
  and 46% more wall time across six tasks, mostly the extra prepare, approve and
  apply commands.
- **The guarantees held under agent use.** No arm W agent wrote the checkout;
  every applied edit carries a verified approval, a durable claim and a recorded
  outcome that `show` re-validates.
- **The prepare/approve split caught one mistake.** In T4 the agent's first
  recipe used `after <line> insert lines`, which joins two lines (the Splicery
  trap found in the first pilot). The agent did not approve that run, prepared
  a corrected one, and applied only that. The rejected preparation never touched
  a file and remains visible in the ledger. One case is an anecdote, not a rate.

## Limits

One repeat per cell; differences this size could be noise. Token totals are
Claude Code's per-subagent usage reports and include fixed host context. The
agent approving its own edit removes the human review the flow is designed
for, so this pilot measures mechanics and overhead, not review quality. The
model alias does not expose an exact backend revision.

## Next

The cases where governance should pay are not single-file edits by a careful
agent: they are concurrent changes between preparation and apply, edits a
reviewer rejects, interrupted applies, and multi-session work. Those need a
two-phase harness (prepare, then an injected change or a reviewer decision,
then apply) and repeated trials.

Reproduce: per trial, copy [`../2026-11-08-edit-pilot/fixture/`](../2026-11-08-edit-pilot/fixture/)
into `T<n>-<arm>/repo` and commit it, run `writfold trial init --state T<n>-W/state`
for arm W, give the agent its prompt with `<trial>`, `<writfold>` and
`<splicery checkout>` filled in, then run
`python3 grade.py ../2026-11-08-edit-pilot/fixture ../2026-11-08-edit-pilot <trials-dir> <writfold>`.
