#!/usr/bin/env bash
# After a trial's agent has finished: record the checkout's final diff and
# status against the pinned commit (so a commit made during the trial cannot
# hide a change), and copy the agent's transcript.
# Usage: collect.sh SCRATCH TRIAL TRANSCRIPT_JSONL
set -euo pipefail
S=$1 t=$2 tr=$3 C=b5addb64f0161ff6bfe94c124ef76f6a1fba5254
d=$S/trials/$t
cd "$d/repo"
git diff $C > "$d/final.diff"
{ git diff --name-status $C | awk -F'\t' '{print " M " $NF}'; git ls-files --others --exclude-standard | sed 's/^/?? /'; } > "$d/status.txt"
git log --oneline $C..HEAD > "$d/commits.txt" || true
mkdir -p "$S/transcripts"
cp "$tr" "$S/transcripts/$t.jsonl"
echo "$t diff_lines=$(wc -l < "$d/final.diff") status_lines=$(wc -l < "$d/status.txt") commits=$(wc -l < "$d/commits.txt") answer=$(test -f "$d/answer.txt" && wc -l < "$d/answer.txt" || echo none)"
