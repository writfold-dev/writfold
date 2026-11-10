# `trial/run` and `trial/evidence-*` latency through `serve`, 10 November 2026

Budget (`docs/writfold/agent-boundary-plan.md`): 5 s ceiling per call, 1 s p95
target, 250 ms for read-only views. Measured request to response with
`python3 -I bench/latency/serve_run_evidence.py <binary> <out> 100000 10` on a
release host build (`cargo build --release -p writfold`, not the gate's
static-musl artifact) of the `serve/trial-run-evidence` working tree, WSL2
x86_64, warm page cache. One `serve` session per repository negotiates
`writfold.trial/1`, then ten times runs `trial/run` (Tuft, a **stub** Weft
and prepare, the same inputs `bench/latency/run.sh` times for `trial run`),
`trial/evidence-list` (default page of 100, and `path: "src/"`) and
`trial/evidence-show` (full digest, and `context_lines: 3` with `repo`). The
large repository's state holds 1,000 bindings retained by Tuft before the
session, as in `bench/latency/evidence.sh`; its last default listing reported
`# count=1001 shown=100 omitted=901`.

| Repository | Method | Median | p95 | Max |
|---|---|---|---|---|
| small | `trial/run` | 167.2 ms | 318.9 ms | 318.9 ms |
| small | `trial/evidence-list` | 0.2 ms | 0.2 ms | 0.2 ms |
| small | `trial/evidence-list` (path) | 0.1 ms | 0.1 ms | 0.1 ms |
| small | `trial/evidence-show` | 0.1 ms | 0.2 ms | 0.2 ms |
| small | `trial/evidence-show` (3 context lines) | 7.7 ms | 8.1 ms | 8.1 ms |
| large | `trial/run` | 331.3 ms | 376.3 ms | 376.3 ms |
| large | `trial/evidence-list` | 19.5 ms | 62.0 ms | 62.0 ms |
| large | `trial/evidence-list` (path) | 17.7 ms | 19.3 ms | 19.3 ms |
| large | `trial/evidence-show` | 0.1 ms | 0.2 ms | 0.2 ms |
| large | `trial/evidence-show` (3 context lines) | 8.3 ms | 9.0 ms | 9.0 ms |

Every call is within budget. With ten samples, p95 is the largest sample.
`trial/run` matches the CLI's `trial run` (median 319 ms on the large
repository in `bench/2026-11-10-trial-run-latency`). The evidence views cost
far less than the CLI's because no process starts per call.

**Not measured:** the real Weft. `trial/run`'s Weft step runs the binary the
client names, for up to its 30 s step limit, so a slow Weft check can exceed
the 5 s ceiling; the plan's rule for such work (return a durable handle at
once) is not implemented here.

Attempts: the first invocation failed before any sample (the script read a
`payload` from `initialize`); the second completed within budget but did not
record the listing trailer, and its files were overwritten by this, the third.
