#!/usr/bin/env bash
# Prepare one trial directory per entry of one stage in order.json: a fresh
# clone of the template at the corpus commit with no remote, an empty work/,
# and for arm B a trial state (writfold trial init, outside the measured
# cost). Writes the filled prompt to SCRATCH/trials/<trial>.prompt.
# Usage: setup.sh SCRATCH TEMPLATE_REPO STAGE   (STAGE: calibration | main;
#        SCRATCH/bin/writfold must exist)
set -euo pipefail
S=$1 T=$2 STAGE=$3 C=b5addb64f0161ff6bfe94c124ef76f6a1fba5254
here=$(cd "$(dirname "$0")/.." && pwd)
for t in $(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1]))[sys.argv[2]]["order"]))' "$here/order.json" "$STAGE"); do
  d=$S/trials/$t; rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"
  git -C "$d/repo" remote remove origin
  test "$(git -C "$d/repo" rev-parse HEAD)" = $C
  case $t in *-B-*) "$S/bin/writfold" trial init --state "$d/state" >/dev/null;; esac
  task=$(echo "$t" | cut -d- -f2) arm=$(echo "$t" | cut -d- -f3)
  sed -e "s|<trial>|$d|g" -e "s|<commit>|$C|g" -e "s|<writfold>|$S/bin/writfold|g" "$here/prompts/$task-$arm.txt" > "$S/trials/$t.prompt"
done
echo "set up $(python3 -c 'import json,sys;print(len(json.load(open(sys.argv[1]))[sys.argv[2]]["order"]))' "$here/order.json" "$STAGE") trials for $STAGE"
