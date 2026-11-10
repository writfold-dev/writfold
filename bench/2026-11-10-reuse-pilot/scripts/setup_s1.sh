#!/usr/bin/env bash
# Prepare the six session-1 directories: a fresh clone at the corpus commit, an
# empty work/ and a trial state (writfold trial init, outside measured cost).
# Usage: setup_s1.sh SCRATCH TEMPLATE_REPO [TASKS]   (SCRATCH/bin/writfold must exist)
# TASKS (default all six) was added after pre-registration to redo R4 and R5
# alone; see the README deviations.
set -euo pipefail
S=$1 T=$2 C=c568505742d00859cad758924c98d8d0a6e98648
here=$(cd "$(dirname "$0")/.." && pwd)
for t in ${3:-R1 R2 R3 R4 R5 R6}; do
  d=$S/s1/$t; rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"; git -C "$d/repo" -c advice.detachedHead=false checkout -q --detach $C
  git -C "$d/repo" remote remove origin
  "$S/bin/writfold" trial init --state "$d/state" >/dev/null
  sed -e "s|<trial>|$d|g" -e "s|<commit>|$C|g" -e "s|<writfold>|$S/bin/writfold|g" "$here/prompts/S1-$t.txt" > "$S/s1/S1-$t.prompt"
done
