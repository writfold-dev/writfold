#!/bin/sh
# Latency of `writfold trial evidence list/show` with 1,000 retained bindings
# on the 100,000-file synthetic repository (bench/latency/repos.py).
# Usage: bench/latency/evidence.sh <writfold-binary> <out-dir> [files=100000] [repeats=20]
# Exits non-zero if any p95 exceeds 250 ms.
set -eu
bin=$(realpath "$1"); out=$(realpath -m "$2"); files=${3:-100000}; repeats=${4:-20}
here=$(dirname "$(realpath "$0")")
mkdir -p "$out"; work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
python3 -I "$here/repos.py" "$work" "$files"
export WRITFOLD_STATE="$work/state"
"$bin" trial init >/dev/null
c=$(git -C "$work/large" rev-parse HEAD)
# 1,000 bindings in 1,000 files: the `fn fN() {}` line of the first 1,000 files.
printf '@h := find rx"^fn f[0-9]+\\(\\)" at "%s" limit 1000;\n@e := retain @h around lines 0;\n@v := project @e [path, line];\nemit @v budget_tokens 100000;\n' "$c" >"$work/retain.tuft"
"$bin" tuft run "$work/retain.tuft" --repo "$work/large" >"$work/retained.txt"
tail -1 "$work/retained.txt"
"$bin" trial evidence list --limit 1 | tail -1
first=$("$bin" trial evidence list --limit 1 | head -1 | cut -d' ' -f1)
full=$("$bin" trial evidence show "$first" | sed -n 's/^# prefix .* resolved to \([0-9a-f]*\).*/\1/p')
"$bin" trial retain --repo "$work/large" --commit "$c" --file src/client.rs --bytes 0..40 --label entry >/dev/null
ms() { s=$(date +%s%N); "$@" >"$work/last" 2>&1 || { cat "$work/last" >&2; exit 1; }; echo $(( ($(date +%s%N) - s) / 1000000 )); }
: >"$out/samples.tsv"
i=0
while [ "$i" -lt "$repeats" ]; do
  printf 'large\tlist\t%s\n' "$(ms "$bin" trial evidence list)" >>"$out/samples.tsv"
  printf 'large\tlist-path\t%s\n' "$(ms "$bin" trial evidence list --path pkg0000/)" >>"$out/samples.tsv"
  rm -f "$work/all.txt"
  printf 'large\tlist-all-out\t%s\n' "$(ms "$bin" trial evidence list --limit 2000 --out "$work/all.txt")" >>"$out/samples.tsv"
  printf 'large\tshow\t%s\n' "$(ms "$bin" trial evidence show "$full")" >>"$out/samples.tsv"
  printf 'large\tshow-prefix\t%s\n' "$(ms "$bin" trial evidence show "$first")" >>"$out/samples.tsv"
  printf 'large\tshow-label\t%s\n' "$(ms "$bin" trial evidence show entry)" >>"$out/samples.tsv"
  printf 'large\tshow-context3\t%s\n' "$(ms "$bin" trial evidence show "$full" --context-lines 3 --repo "$work/large")" >>"$out/samples.tsv"
  i=$((i + 1))
done
python3 -I - "$out/samples.tsv" "$out/results.json" "$files" <<'EOF'
import json, math, sys
rows = {}
for line in open(sys.argv[1]):
    repo, op, ms = line.split("\t")
    rows.setdefault(op, []).append(int(ms))
ops, ok = [], True
for op, xs in rows.items():
    xs.sort()
    p95 = xs[min(len(xs) - 1, math.ceil(0.95 * len(xs)) - 1)]
    ok &= p95 <= 250
    ops.append({"op": op, "samples_ms": xs, "median_ms": xs[len(xs) // 2], "p95_ms": p95, "max_ms": xs[-1]})
    print(f"{op:14} median {xs[len(xs) // 2]:4} ms  p95 {p95:4} ms  max {xs[-1]:4} ms  {'ok' if p95 <= 250 else 'OVER'}")
json.dump({"target_p95_ms": 250, "large_files": int(sys.argv[3]), "retained_bindings": 1001,
           "operations": ops, "within_budget": ok}, open(sys.argv[2], "w"), indent=2)
sys.exit(0 if ok else 1)
EOF
