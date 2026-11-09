#!/bin/sh
# Latency bench for agent-facing trial operations (docs/writfold/agent-boundary-plan.md).
# Usage: bench/latency/run.sh <writfold-binary> <out-dir> [files=100000] [repeats=5]
# Builds a small and a large synthetic repository, runs prepare/approve/apply/show
# `repeats` times on each, and writes results.json. Exits non-zero if any single
# operation exceeds the 5 s ceiling or a p95 exceeds the 1 s target.
set -eu
bin=$(realpath "$1"); out=$(realpath -m "$2"); files=${3:-100000}; repeats=${4:-5}
here=$(dirname "$(realpath "$0")")
mkdir -p "$out"; work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
python3 -I "$here/repos.py" "$work" "$files"
export WRITFOLD_STATE="$work/state"
"$bin" trial init >/dev/null
ms() { s=$(date +%s%N); "$@" >"$work/last" 2>&1 || { cat "$work/last" >&2; exit 1; }; echo $(( ($(date +%s%N) - s) / 1000000 )); }
: >"$out/samples.tsv"
for repo in small large; do
  i=0
  while [ "$i" -lt "$repeats" ]; do
    t=$(ms "$bin" trial prepare --repo "$work/$repo" --file src/client.rs --recipe "$work/edit.spl")
    run=$(grep -o 'run\.[0-9a-f.]*' "$work/last" | head -1)
    printf '%s\tprepare\t%s\n' "$repo" "$t" >>"$out/samples.tsv"
    for op in approve apply show; do
      printf '%s\t%s\t%s\n' "$repo" "$op" "$(ms "$bin" trial "$op" "$run")" >>"$out/samples.tsv"
    done
    i=$((i + 1))
  done
done
python3 -I "$here/summary.py" "$out/samples.tsv" "$out/results.json" "$files"
