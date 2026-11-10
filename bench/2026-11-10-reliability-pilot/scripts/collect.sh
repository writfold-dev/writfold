#!/usr/bin/env bash
# After a trial's agent has finished: record the checkout's final diff and
# status, and copy the agent's transcript.
# Usage: collect.sh SCRATCH TRIAL TRANSCRIPT_JSONL
set -euo pipefail
S=$1 t=$2 tr=$3
d=$S/trials/$t
(cd "$d/repo" && git diff > "$d/final.diff" && git status --porcelain --untracked-files=all > "$d/status.txt")
mkdir -p "$S/transcripts"
cp "$tr" "$S/transcripts/$t.jsonl"
echo "$t diff_lines=$(wc -l < "$d/final.diff") status_lines=$(wc -l < "$d/status.txt") answer=$(test -f "$d/answer.txt" && wc -l < "$d/answer.txt" || echo none)"
