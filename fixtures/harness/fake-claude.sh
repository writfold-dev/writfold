#!/bin/sh
# A stand-in for the `claude` command (#150): it takes the arguments the
# Claude Code adapter passes (`-p --agents JSON --agent ID --tools … --model
# M … PROMPT`), checks the agent definition names the agent, and prints a
# report naming the prompt's first line. No model is called.
set -eu
agent=""
prev=""
for a in "$@"; do
    if [ "$prev" = "--agent" ]; then agent=$a; fi
    prev=$a
    last=$a
done
test -n "$agent"
printf 'Reviewed: %s\nThe two records are consistent.\n' "$(printf '%s\n' "$last" | head -n 1)"
