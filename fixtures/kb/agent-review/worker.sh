#!/bin/sh
# A fake worker for the agent example (#151): it reads the package
# `writfold kb agent prepare` wrote and writes a report. A real harness
# would hand instructions.md and input to a model. Usage: worker.sh PKG OUT
set -eu
pkg=$1
out=$2
test -f "$pkg/instructions.md"
test -f "$pkg/invocation.json"
{
    printf 'Reviewed: %s\n' "$(head -n 1 "$pkg/input")"
    printf 'What holds: the claim. What does not: nothing. Not shown: timings.\n'
} > "$out"
