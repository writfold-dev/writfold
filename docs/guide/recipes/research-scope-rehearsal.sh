#!/bin/sh
# Rehearse the research-scope gate (#155) end to end: a KB with a seed
# ticket and five decisions that reference it, the research-scope.toml gate
# in the KB, one run, and the cases the scope must get right. Each case
# prints a "## " section with what `gate run` and `gate status` said.
#
#   research-scope-rehearsal.sh DIR     DIR must not exist
#
# Needs `writfold` and `git` on PATH. Nothing outside DIR is written.
set -eu
DIR=$1
HERE=$(cd "$(dirname "$0")" && pwd)
[ ! -e "$DIR" ] || { echo "refusing: $DIR exists" >&2; exit 1; }
mkdir -p "$DIR" && DIR=$(cd "$DIR" && pwd)
export GIT_AUTHOR_NAME=rehearsal GIT_AUTHOR_EMAIL=rehearsal@example.invalid
export GIT_COMMITTER_NAME=rehearsal GIT_COMMITTER_EMAIL=rehearsal@example.invalid
unset WRITFOLD_KB WRITFOLD_SESSION WRITFOLD_KB_WORK
say() { printf '\n## %s\n' "$*"; }
show() { printf '$ %s\n' "$*"; "$@" || true; }
run() { show writfold kb gate run "${1:-related-scope}" --run "$RUN" --repo "$DIR/kb"; }
status() { show writfold kb gate status --run "$RUN" --repo "$DIR/kb"; }
members() {
    kept=$(ls -td runs/"$RUN"/evidence/related-scope-* | head -1)/manifest.json
    printf 'members=%s %s\n' "$(grep -c '"path"' "$kept")" "$(grep '"coverage"' "$kept" | tr -d ' ,')"
}

say "setup: a seed, five decisions that reference it, the gate, a run"
git init -q -b main "$DIR/kb"
cd "$DIR/kb"
writfold kb init >/dev/null
writfold kb record ticket SEED-1 --title "Flush latency budget" >/dev/null
for x in a b c d e; do
    writfold kb record decision --title "Decision $x" --target-source "src/$x.rs" --references SEED-1 >/dev/null
done
mkdir -p .writfold/gates
cp "$HERE/research-scope.toml" .writfold/gates/related-scope.toml
writfold kb checkpoint --new-run --project kb --task "scope rehearsal" --note "started" >/dev/null
RUN=$(ls runs | head -1)
git add -A && git commit -qm "KB with a seed, five decisions and the scope gate"

say "scope read in full"
run
members

say "walk stopped at its work limit"
WRITFOLD_KB_WORK=2 run

say "walk timed out"
sed -e 's/^id = .*/id = "slow-scope"/' -e 's/^timeout_seconds = .*/timeout_seconds = 1/' \
    -e 's/^argv = \["writfold"/argv = ["sh", "-c", "sleep 5; exec \\"$0\\" \\"$@\\"", "writfold"/' \
    .writfold/gates/related-scope.toml > .writfold/gates/slow-scope.toml
git add -A && git commit -qm "a slow copy of the scope gate"
run slow-scope
rm .writfold/gates/slow-scope.toml && git add -A && git commit -qm "drop the slow copy"

say "scope read in full again"
run

say "a record added that references the seed"
writfold kb record decision --title "Decision f" --target-source src/f.rs --references SEED-1 >/dev/null
status
run
members

say "fresh clone with the evidence"
git add -A && git commit -qm "run with its scope receipts and evidence"
git clone -q "$DIR/kb" "$DIR/clone"
cd "$DIR/clone"
show writfold kb gate status --run "$RUN" --repo "$DIR/clone"

say "fresh clone without the evidence"
rm -r runs/*/evidence
show writfold kb gate status --run "$RUN" --repo "$DIR/clone"

say "a seed nothing references"
cd "$DIR/kb"
writfold kb record ticket LONE-1 --title "Nothing points here" >/dev/null
show writfold kb impact LONE-1 --direction in --depth 1 --include inferred --manifest "$DIR/lone.json"
printf 'members=%s %s\n' "$(grep -c '"path"' "$DIR/lone.json")" "$(grep '"coverage"' "$DIR/lone.json" | tr -d ' ,')"
