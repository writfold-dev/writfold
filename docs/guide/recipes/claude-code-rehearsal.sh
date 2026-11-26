#!/bin/sh
# Drive the research contract through the Claude Code adapter (#150): a seed
# and two records that reference it, the definitions of
# fixtures/kb/kb-consistency/, the scope read, then one unit prepared,
# exported, run by the harness, delivered bound to its invocation, checked
# and covered. Each stage prints a "## " section.
#
#   claude-code-rehearsal.sh DIR [HARNESS]   DIR must not exist
#
# HARNESS is the command the adapter runs (default: the fake harness in
# fixtures/harness/fake-claude.sh; pass `claude` for the real CLI, which
# then needs `--model` through ADAPTER_MODEL, e.g. haiku). Needs
# `writfold`, `git` and `python3` on PATH. Nothing outside DIR is written.
set -eu
DIR=$1
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
HARNESS=${2:-"sh $ROOT/fixtures/harness/fake-claude.sh"}
ADAPTER="python3 $ROOT/plugins/writfold/adapter/claude_code.py"
[ ! -e "$DIR" ] || { echo "refusing: $DIR exists" >&2; exit 1; }
mkdir -p "$DIR" && DIR=$(cd "$DIR" && pwd)
export GIT_AUTHOR_NAME=rehearsal GIT_AUTHOR_EMAIL=rehearsal@example.invalid
export GIT_COMMITTER_NAME=rehearsal GIT_COMMITTER_EMAIL=rehearsal@example.invalid
unset WRITFOLD_KB WRITFOLD_SESSION WRITFOLD_KB_WORK
say() { printf '\n## %s\n' "$*"; }
show() { printf '$ %s\n' "$*"; "$@" 2>&1 || true; }
at() { echo --run "$RUN" --repo "$DIR/kb"; }
first() { "$@" 2>/dev/null | head -n 1 | awk '{print $2}'; }
model() { if [ -n "${ADAPTER_MODEL:-}" ]; then echo --model "$ADAPTER_MODEL"; fi; }

say "setup: a seed, two records, the definitions, a run, the scope read"
git init -q -b main "$DIR/kb"
cd "$DIR/kb"
writfold kb init >/dev/null
writfold kb record ticket SEED-1 --title "Flush latency budget" >/dev/null
writfold kb record decision --title "Flush within 5 ms" --target-source src/a.rs --references SEED-1 >/dev/null
writfold kb record decision --title "Flush within 50 ms" --target-source src/b.rs --references SEED-1 >/dev/null
cp -R "$ROOT/fixtures/kb/kb-consistency/.writfold" .
writfold kb checkpoint --new-run --project kb --task "adapter rehearsal" --note "started" >/dev/null
RUN=$(ls runs | head -1)
git add -A && git commit -qm "KB, definitions and a run"
writfold kb plan set kb-consistency $(at) >/dev/null
writfold kb gate run related-scope $(at) >/dev/null
writfold kb plan advance $(at) >/dev/null
KEPT=$(ls -td runs/"$RUN"/evidence/related-scope-* | head -1)/manifest.json
IDS=$(sed -n 's/^ *"id": "\(.*\)",*$/\1/p' "$KEPT" | sort)
UNIT=$(echo $IDS | tr ' ' '~')
PATHS=$(sed -n 's/^ *"path": "\(.*\)",*$/\1/p' "$KEPT")
printf 'unit=%s\n' "$UNIT"

say "prepare"
printf '%s\n%s\n' "$UNIT" "$PATHS" > "$DIR/input"
INV=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input" --out "$DIR/pkg" \
    --for "pair-review.$UNIT" $(at))
printf 'invocation=%s\n' "$INV"

say "export"
$ADAPTER export "$DIR/pkg" $(model) > "$DIR/agents.json"
$ADAPTER export "$DIR/pkg" $(model) | cmp -s - "$DIR/agents.json" && echo "export=deterministic"
cat "$DIR/agents.json"

say "run"
$ADAPTER run "$DIR/pkg" --report "$DIR/report.md" --harness "$HARNESS" $(model)
cat "$DIR/report.md"

say "deliver"
$ADAPTER deliver "$DIR/pkg" --mode pair-report --report "$DIR/report.md" --run "$RUN" 2>&1
CAND=$(ls -t runs/"$RUN"/deliveries | head -1)

say "cover"
show writfold kb gate cover pair-review "$UNIT" supported "delivery:$CAND" --by adapter $(at)
show writfold kb gate status --budget-tokens 20000 $(at)

say "a statement the adapter can make"
$ADAPTER deliver "$DIR/pkg" --mode pair-report-strict --report "$DIR/report.md" --run "$RUN" \
    --attest context.submitted 2>&1 | grep -E "^delivery|context.submitted" || true
