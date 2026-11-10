#!/usr/bin/env bash
# Prepare one session-2 trial directory per entry in order.json, after session 1:
# a fresh clone at the corpus commit and an empty work/; arm A gets a copy of
# session 1's notes.txt, arm B a copy of session 1's trial state (no notes).
# Usage: setup_s2.sh SCRATCH TEMPLATE_REPO
set -euo pipefail
S=$1 T=$2 C=c568505742d00859cad758924c98d8d0a6e98648
here=$(cd "$(dirname "$0")/.." && pwd)
for t in $(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1]))["order"]))' "$here/order.json"); do
  task=${t%%-*}; d=$S/trials/$t; rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"; git -C "$d/repo" -c advice.detachedHead=false checkout -q --detach $C
  git -C "$d/repo" remote remove origin
  case $t in
    *-A-*) cp "$S/s1/$task/notes.txt" "$d/notes.txt";;
    *-B-*) cp -a "$S/s1/$task/state" "$d/state";;
  esac
  sed -e "s|<trial>|$d|g" -e "s|<commit>|$C|g" -e "s|<writfold>|$S/bin/writfold|g" "$here/prompts/${t%-r*}.txt" > "$S/trials/$t.prompt"
done
