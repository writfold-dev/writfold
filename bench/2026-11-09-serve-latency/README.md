# `writfold.trial/1` latency through `serve`, 9 November 2026

Budget (`docs/writfold/agent-boundary-plan.md`): 5 s ceiling per call, 1 s p95
target, 250 ms for read-only `trial/show`. Measured request to response with
`python3 -I bench/latency/serve.py <binary> <out> 100000 10` on a release host
build (`cargo build --release -p writfold`, not the gate's static-musl artifact)
from the `serve/trial` branch source, WSL2 x86_64, warm page cache. One `serve`
session per repository negotiates `writfold.trial/1`, then runs
retain → prepare → show ten times on the same 3-file and 100,000-file
synthetic repositories as `bench/latency/run.sh`.

| Repository | Method | Median | p95 | Max |
|---|---|---|---|---|
| small | `trial/retain` | 37.4 ms | 41.3 ms | 41.3 ms |
| small | `trial/prepare` | 163.5 ms | 184.3 ms | 184.3 ms |
| small | `trial/show` | 51.5 ms | 53.1 ms | 53.1 ms |
| large | `trial/retain` | 15.0 ms | 26.8 ms | 26.8 ms |
| large | `trial/prepare` | 137.4 ms | 165.5 ms | 165.5 ms |
| large | `trial/show` | 50.8 ms | 53.0 ms | 53.0 ms |

Every call is within budget. With ten samples, p95 is the largest sample. A
session's `show` costs less than the CLI's (161 ms in
`bench/2026-11-09-latency`) mainly because no process starts per call.
