# Trial operation latency, 9 November 2026

Budget (`docs/writfold/agent-boundary-plan.md`): each agent-facing operation must
finish within 5 s, with a 1 s p95 target. Measured with `bench/latency/run.sh` on the
release binary (WSL2, x86_64, warm page cache), with 5 repeats per operation on a
3-file and a 100,000-file (≈200 MB) synthetic repository.

Before the sparse worktree (single runs): `prepare` took 183 ms on the small repository and
**7,772 ms** on the large one, because `git worktree add` checked out the whole
tree. `approve` took 196 ms, `apply` 431 ms and `show` 167 ms.

After: see `results.json` and `samples.tsv`. On the large repository, `prepare`
has a median of 163 ms and a p95 of 239 ms. Every operation is within budget.
