#!/bin/sh
# An outer loop for unattended use (docs/guide/run-graph.md, "Loops"): one
# fresh agent session at a time until the run's plan is finished or its
# current loop is spent. Writfold runs nothing here: this script is the
# loop. Writfold records each round the agent takes (`writfold kb plan
# again`) and refuses one past the loop's max.
#
#   loop-rounds.sh [MAX_SESSIONS] -- AGENT COMMAND...
#   loop-rounds.sh 20 -- claude -p "Follow the next: line of writfold kb plan status."
#
# Exit 0: the plan reads complete (or complete-with-exceptions); 2: the
# current loop is spent, stop and report; 3: MAX_SESSIONS ran out.
set -u
max=${1:-20}
[ "$#" -gt 0 ] && shift
[ "${1:-}" = "--" ] && shift
i=0
while [ "$i" -lt "$max" ]; do
    status=$(writfold kb plan status 2>&1)
    case "$status" in
    *" summary=complete "* | *" summary=complete-with-exceptions "*)
        echo "plan complete"
        exit 0
        ;;
    esac
    if printf '%s\n' "$status" | grep -q '!spent$'; then
        echo "loop spent: stop and report"
        printf '%s\n' "$status" | grep -A2 '!spent$'
        exit 2
    fi
    "$@" || exit $?
    i=$((i + 1))
done
echo "stopped after $max sessions"
exit 3
