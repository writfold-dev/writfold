#!/usr/bin/env bash
# Prepare one trial directory per entry in order.json: a fresh clone at the
# corpus commit with no remote, an empty work/, and for arm B a trial state
# (writfold trial init, outside the measured cost). Writes the filled prompt
# to SCRATCH/trials/<trial>.prompt.
# Usage: setup.sh SCRATCH TEMPLATE_REPO      (SCRATCH/bin/writfold must exist)
set -euo pipefail
S=$1 T=$2 C=e56621f7acf0d3392add5d814c35134b66c872a7
here=$(cd "$(dirname "$0")/.." && pwd)
for t in $(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1]))["order"]))' "$here/order.json"); do
  d=$S/trials/$t; rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"
  git -C "$d/repo" remote remove origin
  test "$(git -C "$d/repo" rev-parse HEAD)" = $C
  case $t in *-B-*) "$S/bin/writfold" trial init --state "$d/state" >/dev/null;; esac
  sed -e "s|<trial>|$d|g" -e "s|<commit>|$C|g" -e "s|<writfold>|$S/bin/writfold|g" "$here/prompts/${t%-r*}.txt" > "$S/trials/$t.prompt"
done
