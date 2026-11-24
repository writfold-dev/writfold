#!/bin/sh
# An optional Plyrune check as an ordinary Writfold command gate (#106;
# docs/guide/gates.md, "An external checker as a gate"). Copy this file next
# to the gate definition and name it through {gate}:
#
#   # .writfold/gates/plyrune.toml
#   schema = "writfold-gate/1"
#   id = "plyrune"
#   version = 1
#   class = "mechanical"
#   argv = ["sh", "{gate}/plyrune-check.sh", "{artifacts}", "requirements/r1.md"]
#   timeout_seconds = 300
#   scope = ["src/**", "requirements/**"]
#   retain = ["report.json"]
#
# This script is the adapter boundary. Its bytes are part of the gate's
# definition digest, so editing it (or the pin below) makes earlier
# receipts stale. It:
#   - exits 127 when the checker is not on PATH: the gate is owed, with
#     this message, and nothing is installed or downloaded;
#   - refuses any checker version other than PIN (exit 3);
#   - passes explicit inputs: the requirement, the base commit and the
#     candidate (the work tree at the gate's subject);
#   - copies the checker's own report, unchanged, to {artifacts}/report.json,
#     which `retain` keeps with the receipt. With no report the gate is
#     `unknown` whatever the exit status; a failure stays a failure.
#
# Writfold does not read the report yet: checking its profile and subject
# against the gate's subject waits for the exchange profile agreed in
# plyrune-dev/plyrune#39 (writfold#122). Until then a pass means: the pinned
# checker exited 0 and its report was kept. The command line below is a
# stand-in for the one plyrune#30 will fix; change it and PIN together.
set -u
PIN=${PLYRUNE_PIN:-"plyrune 0.0.0-standin"}
PLYRUNE=${PLYRUNE:-plyrune}
artifacts=$1
requirement=$2

if ! command -v "$PLYRUNE" >/dev/null 2>&1; then
  echo "plyrune-check: $PLYRUNE is not on PATH; install the pinned checker ($PIN) to meet this gate" >&2
  exit 127
fi
version=$("$PLYRUNE" --version 2>/dev/null)
if [ "$version" != "$PIN" ]; then
  echo "plyrune-check: checker is '$version', this gate is pinned to '$PIN'" >&2
  exit 3
fi
[ -f "$requirement" ] || { echo "plyrune-check: no requirement at $requirement" >&2; exit 2; }
base=$(git rev-parse HEAD) || exit 2
"$PLYRUNE" check --requirement "$requirement" --base "$base" --candidate . \
  --report "$artifacts/report.json"
