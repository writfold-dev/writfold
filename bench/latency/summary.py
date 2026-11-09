"""Summarise latency samples; fail if any sample exceeds 5 s or a p95 exceeds 1 s."""
import json
import math
import sys

samples, out, files = sys.argv[1], sys.argv[2], int(sys.argv[3])
CEILING_MS, TARGET_P95_MS = 5000, 1000
rows: dict[tuple[str, str], list[int]] = {}
for line in open(samples):
    repo, op, ms = line.split("\t")
    rows.setdefault((repo, op), []).append(int(ms))
result = {"ceiling_ms": CEILING_MS, "target_p95_ms": TARGET_P95_MS, "large_files": files, "operations": []}
ok = True
for (repo, op), xs in sorted(rows.items()):
    xs.sort()
    p95 = xs[min(len(xs) - 1, math.ceil(0.95 * len(xs)) - 1)]
    within = xs[-1] <= CEILING_MS and p95 <= TARGET_P95_MS
    ok &= within
    result["operations"].append({"repo": repo, "op": op, "samples_ms": xs, "median_ms": xs[len(xs) // 2],
                                 "p95_ms": p95, "max_ms": xs[-1], "within_budget": within})
    print(f"{repo:6} {op:8} median {xs[len(xs) // 2]:5} ms  p95 {p95:5} ms  max {xs[-1]:5} ms  {'ok' if within else 'OVER'}")
result["within_budget"] = ok
json.dump(result, open(out, "w"), indent=2)
sys.exit(0 if ok else 1)
