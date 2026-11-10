#!/usr/bin/env bash
# Prepare one trial directory per entry in order.json.
# Usage: setup.sh SCRATCH TEMPLATE_REPO   (SCRATCH/bin/writfold must exist)
# TEMPLATE_REPO is a clone made before the pre-registration commit, detached at
# the pinned commit, with no remote and no branches, so trials cannot see later
# commits (and therefore not expected.json).
set -euo pipefail
S=$1 T=$2 C=c568505742d00859cad758924c98d8d0a6e98648
here=$(cd "$(dirname "$0")/.." && pwd)
for t in $(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1]))["order"]))' "$here/order.json"); do
  d=$S/trials/$t; rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"; git -C "$d/repo" -c advice.detachedHead=false checkout -q --detach $C
  git -C "$d/repo" remote remove origin
  case $t in *-B-*) "$S/bin/writfold" trial init --state "$d/state" >/dev/null;; esac
  sed -e "s|<trial>|$d|g" -e "s|<commit>|$C|g" -e "s|<writfold>|$S/bin/writfold|g" "$here/prompts/${t%-r*}.txt" > "$S/trials/$t.prompt"
done
