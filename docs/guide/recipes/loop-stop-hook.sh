#!/bin/sh
# A hard Stop hook for a session walking a loop (docs/guide/hooks.md): it
# blocks the end of the turn while the current loop step has a round left,
# and lets the turn end on a spent loop, a finished plan or anything else.
# Writfold judges; this hook only reads `writfold kb plan status`. Register
# it as a Claude Code Stop hook; it reads and ignores the event on stdin.
cat >/dev/null
status=$(writfold kb plan status 2>/dev/null) || exit 0
case "$status" in
*"next: writfold kb plan again"*)
    printf '%s\n' '{"decision": "block", "reason": "The current loop has a round left: run writfold kb plan again, then follow its next: line."}'
    ;;
esac
exit 0
