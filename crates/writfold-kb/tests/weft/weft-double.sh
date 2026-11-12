#!/bin/sh
# TEST DOUBLE for the weft binary, used by tests/gate_assessed.rs only.
# It does not reason: it prints the report the real Weft printed for the
# same source (step.report.json, recorded from weft prove --json), with
# the source digest filled in for the bytes it was given. Called as
# `weft prove --json -- <source>`. One test runs the real binary instead.
set -eu
here=$(dirname "$0")
name=$(basename "$4" .weft)
sha=$(sha256sum "$4" | cut -d' ' -f1)
sed "s/@SHA@/$sha/" "$here/$name.report.json"
exit 1
