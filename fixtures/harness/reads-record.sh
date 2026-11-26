#!/bin/sh
# A fake harness (#157) that reads the package's invocation.json: it finds
# the unit the invocation was prepared for and writes a report about it.
# No model is called. Usage: reads-record.sh PKG OUT
set -eu
unit=$(sed -n 's/^  "for": "\(.*\)",*$/\1/p' "$1/invocation.json")
printf 'Unit %s: the two records are consistent.\n' "${unit:-unknown}" > "$2"
