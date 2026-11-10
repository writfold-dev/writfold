# `trial run` latency (milestone T1), 10 November 2026

`bench/latency/run.sh target/release/writfold <this dir> 100000 5` on WSL2
(release host build, warm cache, 5 repeats). Weft is a stub that prints a fixed
`weft.report/1` citing the source's first `citation`; the real Weft is not
timed here. `trial-run` is the one command; `part-*` run its parts one by
one: `tuft run` on the same program, the stub on the same substituted source,
and `trial prepare --report`.

| Repository | `trial-run` median / p95 | parts, summed median / p95 |
|---|---|---|
| small (3 files) | 188 / 215 ms | 178 / 245 ms (tuft 37/73, weft 6/11, prepare 135/161) |
| large (100,000 files) | 319 / 487 ms | 314 / 355 ms (tuft 171/180, weft 4/7, prepare 139/168) |

Medians match the sum of the parts within 10 ms; the command adds no step of
its own beyond writing the substituted source and report. Every `trial-run`
sample is under 0.5 s.

The script exited 1: on the large repository two existing Tuft rows,
`tuft-rx` (p95 1,285 ms) and `tuft-retain50` (p95 1,376 ms), were over the
1 s p95 target in this run (earlier recorded at 798 and 759 ms). Those
programs did not change in this milestone; the overrun is recorded, not
explained, and is not part of `trial run`.
