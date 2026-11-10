# Evidence read-back latency, 10 November 2026

Budget for `writfold trial evidence list/show`: p95 at most 250 ms on the
100,000-file synthetic repository (`bench/latency/repos.py`) with 1,000
retained bindings. Measured with
[`bench/latency/evidence.sh`](../latency/evidence.sh) on a release build of the
commit that added the commands (WSL2, x86_64, warm page cache), 20 repeats per
operation.

Setup: a Tuft program retains the `fn fN() {}` line of the first 1,000 files
(`around lines 0`, 1,000 bindings in 1,000 files), and one `trial retain
--label entry` adds a 1,001st binding with a label. Every `list` reads,
decodes, re-fingerprints and digest-checks all 1,001 entries before filtering.

| Operation | Median | p95 | Max |
|---|---|---|---|
| `evidence list` (default `--limit 100`) | 36 ms | 97 ms | 99 ms |
| `evidence list --path pkg0000/` | 37 ms | 97 ms | 160 ms |
| `evidence list --limit 2000 --out FILE` (all 1,001 rows) | 54 ms | 128 ms | 175 ms |
| `evidence show <digest>` | 2 ms | 6 ms | 11 ms |
| `evidence show <12-hex prefix>` | 5 ms | 16 ms | 24 ms |
| `evidence show <label>` | 2 ms | 6 ms | 27 ms |
| `evidence show <digest> --context-lines 3 --repo R` | 20 ms | 72 ms | 91 ms |

Every p95 is within 250 ms. Samples are in [`samples.tsv`](samples.tsv) and
the summary in [`results.json`](results.json). `list` is linear in the number
of retained bindings, since it verifies every entry; `show --context-lines`
adds one pinned-blob read through the admitted Git executable.
