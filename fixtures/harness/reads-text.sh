#!/bin/sh
# A fake harness (#157) of another shape: it reads only instructions.md and
# input, never invocation.json, and writes a report. No model is called.
# Usage: reads-text.sh PKG OUT
set -eu
test -s "$1/instructions.md"
printf 'Reviewed: %s\nThe two records are consistent.\n' "$(head -n 1 "$1/input")" > "$2"
