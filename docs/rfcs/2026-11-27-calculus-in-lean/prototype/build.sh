#!/bin/sh
# Prototype of docs/rfcs/2026-11-27-calculus-in-lean.md. Built by no gate.
# How it was compiled on 27 November 2026: copy this directory to /tmp/calc
# in a container of the proofs image that holds a built /work/proofs
# (docs/writfold/aeneas.md, "To iterate without rebuilding"), then
#   sh /tmp/calc/build.sh Calculus/Word Calculus/Account Calculus/Evidence \
#      Calculus/Site Calculus/Referent Conform/Sample Conform/Cover
# Each file is compiled against the image's Lean, Aeneas backend and the
# built Extracted and Writfold libraries; nothing under /work is written.
cd /tmp/calc
export LEAN_PATH="$(cd /work/proofs && lake env printenv LEAN_PATH):/tmp/calc/out"
for f in "$@"; do
  mkdir -p "out/$(dirname "$f")"
  timeout 300 lean -o "out/$f.olean" "$f.lean" > "out/$f.log" 2>&1 </dev/null
  echo "== $f exit=$? $(grep -c 'error' out/$f.log) errors"
  head -c 3000 "out/$f.log"
done
