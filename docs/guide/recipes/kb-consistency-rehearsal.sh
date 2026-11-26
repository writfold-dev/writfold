#!/bin/sh
# Rehearse the KB-consistency research contract (#157) end to end with two
# fake harnesses and no model: a seed and five records that reference it,
# the definitions of fixtures/kb/kb-consistency/, and every attempt to skip
# required work, each printed as a "## " section with what refused it.
#
#   kb-consistency-rehearsal.sh DIR     DIR must not exist
#
# Needs `writfold` and `git` on PATH. Nothing outside DIR is written.
set -eu
DIR=$1
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
[ ! -e "$DIR" ] || { echo "refusing: $DIR exists" >&2; exit 1; }
mkdir -p "$DIR" && DIR=$(cd "$DIR" && pwd)
export GIT_AUTHOR_NAME=rehearsal GIT_AUTHOR_EMAIL=rehearsal@example.invalid
export GIT_COMMITTER_NAME=rehearsal GIT_COMMITTER_EMAIL=rehearsal@example.invalid
unset WRITFOLD_KB WRITFOLD_SESSION WRITFOLD_KB_WORK
say() { printf '\n## %s\n' "$*"; }
show() { printf '$ %s\n' "$*"; "$@" 2>&1 || true; }
at() { echo --run "$RUN" --repo "$DIR/kb"; }
KB="$DIR/kb"
first() { "$@" 2>/dev/null | head -n 1 | awk '{print $2}'; }

# prepare UNIT N [HARNESS]: an invocation --for the unit, its package
# run by a fake harness, the report delivered and checked; prints the
# candidate id.
prepare() {
    # The unit, then each member's file from the kept manifest: a worker
    # cannot read a record from its id alone (found by the #150 hand run).
    { printf '%s\n' "$1"; for id in $(echo "$1" | tr '~' ' '); do path_of "$id"; done; } > "$DIR/input-$2"
    inv=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input-$2" \
        --out "$DIR/pkg-$2" --for "pair-review.$1" $(at))
    sh "$ROOT/fixtures/harness/${3:-reads-record}.sh" "$DIR/pkg-$2" "$DIR/report-$2.md"
    cand=$(first writfold kb deliver prepare --mode pair-report --from "$DIR/report-$2.md" \
        --invocation "$inv" --run "$RUN")
    writfold kb deliver check "$cand" --run "$RUN" >/dev/null 2>&1 || true
    echo "$cand"
}
# The file of member ID in the kept manifest.
path_of() { awk -v id="$1" '$0 ~ "\"id\": \"" id "\"" { getline; sub(/.*"path": "/, ""); sub(/".*/, ""); print }' "$KEPT"; }
cover() { show writfold kb gate cover pair-review "$1" supported "delivery:$2" --by agent $(at); }
status() { show writfold kb gate status --budget-tokens 20000 $(at); }

say "setup: a seed, five records that reference it, the definitions, a run"
git init -q -b main "$KB"
cd "$KB"
writfold kb init >/dev/null
writfold kb record ticket SEED-1 --title "Flush latency budget" >/dev/null
for x in a b c d e; do
    writfold kb record decision --title "Decision $x" --target-source "src/$x.rs" --references SEED-1 >/dev/null
done
cp -R "$ROOT/fixtures/kb/kb-consistency/.writfold" .
writfold kb checkpoint --new-run --project kb --task "consistency review" --note "started" >/dev/null
RUN=$(ls runs | head -1)
git add -A && git commit -qm "KB, definitions and a run"
writfold kb plan set kb-consistency $(at) >/dev/null

say "prepare before the scope is read"
printf 'x\n' > "$DIR/input-x"
show writfold kb agent prepare consistency-reviewer --input "$DIR/input-x" --out "$DIR/pkg-x" $(at)

say "scope walk stopped at its work limit"
WRITFOLD_KB_WORK=2 show writfold kb gate run related-scope $(at)
show writfold kb agent prepare consistency-reviewer --input "$DIR/input-x" --out "$DIR/pkg-x" $(at)

say "enumerate"
show writfold kb gate run related-scope $(at)
KEPT=$(ls -td runs/"$RUN"/evidence/related-scope-* | head -1)/manifest.json
IDS=$(sed -n 's/^ *"id": "\(.*\)",*$/\1/p' "$KEPT" | sort)
UNITS=$(for a in $IDS; do for b in $IDS; do if [ "$a" \< "$b" ]; then echo "$a~$b"; fi; done; done)
printf 'units=%s\n' "$(echo "$UNITS" | wc -l | tr -d ' ')"
# An invocation prepared now, at the enumerate step, for a later report.
EARLY=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input-x" \
    --out "$DIR/pkg-early" --for "pair-review.$(echo "$UNITS" | head -n 1)" $(at))
show writfold kb plan advance $(at)

say "ten prepares and ten reports, two harnesses"
n=0
CANDS=""
for u in $UNITS; do
    n=$((n + 1))
    # The harness switches half way: new invocations, earlier reports stay.
    h=reads-record; [ $n -gt 5 ] && h=reads-text
    CANDS="$CANDS $(prepare "$u" "$n" "$h")"
done
show writfold kb deliver status --run "$RUN"
unit() { echo "$UNITS" | sed -n "${1}p"; }
cand() { echo $CANDS | awk -v i="$1" '{print $i}'; }

say "eight of ten covered"
for i in 1 2 3 4 5 6 7 8; do cover "$(unit $i)" "$(cand $i)" | grep -E "^writfold kb: refused|^\$" || true; done
status
show writfold kb plan advance $(at)

say "a report that names no invocation"
printf 'No invocation.\n' > "$DIR/report-none.md"
NONE=$(first writfold kb deliver prepare --mode pair-report --from "$DIR/report-none.md" --run "$RUN")
show writfold kb deliver check "$NONE" --run "$RUN"
cover "$(unit 9)" "$NONE"

say "a report prepared at another step"
printf 'Early.\n' > "$DIR/report-early.md"
STEPPED=$(first writfold kb deliver prepare --mode pair-report --from "$DIR/report-early.md" --invocation "$EARLY" --run "$RUN")
show writfold kb deliver check "$STEPPED" --run "$RUN"
cover "$(unit 9)" "$STEPPED"

say "a report prepared for another unit"
cover "$(unit 9)" "$(cand 1)"

say "--out inside the KB"
show writfold kb agent prepare consistency-reviewer --input "$DIR/input-x" --out "$KB/pkg-in" --for "pair-review.$(unit 1)" $(at)

say "--for an unknown unit"
show writfold kb agent prepare consistency-reviewer --input "$DIR/input-x" --out "$DIR/pkg-unknown" --for "pair-review.nope~nada" $(at)

say "the skill edited after prepare"
printf '%s\n' "$(unit 9)" > "$DIR/input-edit"
EDITED_INV=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input-edit" \
    --out "$DIR/pkg-edit" --for "pair-review.$(unit 9)" $(at))
sed -i.bak 's/quote the lines that disagree/quote what disagrees/' .writfold/skills/kb-consistency.toml
sh "$ROOT/fixtures/harness/reads-text.sh" "$DIR/pkg-edit" "$DIR/report-edit.md"
EDITED=$(first writfold kb deliver prepare --mode pair-report --from "$DIR/report-edit.md" --invocation "$EDITED_INV" --run "$RUN")
show writfold kb deliver check "$EDITED" --run "$RUN"
mv .writfold/skills/kb-consistency.toml.bak .writfold/skills/kb-consistency.toml

say "a statement the harness cannot make"
printf 'strict\n' > "$DIR/input-strict"
STRICT_INV=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input-strict" \
    --out "$DIR/pkg-strict" --for "pair-review.$(unit 10)" $(at))
sh "$ROOT/fixtures/harness/reads-record.sh" "$DIR/pkg-strict" "$DIR/report-strict.md"
STRICT=$(first writfold kb deliver prepare --mode pair-report-strict --from "$DIR/report-strict.md" --invocation "$STRICT_INV" --run "$RUN")
show writfold kb deliver status "$STRICT" --run "$RUN"

say "a wrong report that is well formed"
printf 'Every claim in both records is false.\n' > "$DIR/report-wrong.md"
WRONG_INV=$(first writfold kb agent prepare consistency-reviewer --input "$DIR/input-edit" \
    --out "$DIR/pkg-wrong" --for "pair-review.$(unit 9)" $(at))
WRONG=$(first writfold kb deliver prepare --mode pair-report --from "$DIR/report-wrong.md" --invocation "$WRONG_INV" --run "$RUN")
show writfold kb deliver check "$WRONG" --run "$RUN"
cover "$(unit 9)" "$WRONG"

say "one unit, both harnesses"
A=$(prepare "$(unit 1)" 21 reads-record)
B=$(prepare "$(unit 1)" 22 reads-text)
for c in "$A" "$B"; do
    writfold kb deliver status "$c" --run "$RUN" 2>&1 | grep -E "^delivery|invocation.bound" | sed -e "s/$c/CAND/" -e 's/ (invocation [^)]*)//' -e 's/ for [0-9a-f]*//' -e 's/ candidate=[0-9a-f]*//' -e 's/ bytes=[0-9]*//' -e 's/ @.*//'
done

say "all covered, the findings disposed of, the plan complete"
cover "$(unit 10)" "$(cand 10)" >/dev/null
show writfold kb plan advance $(at)
status
show writfold kb gate attest findings-disposition --statement "no inconsistency found; nothing to fix" --by owner $(at)
show writfold kb plan advance $(at)
show writfold kb plan status $(at)

say "a report checked again after its step is done"
show writfold kb deliver check "$(cand 1)" --run "$RUN"
status

say "a report withdrawn after it covered a unit"
show writfold kb deliver withdraw "$(cand 2)" --reason superseded --by owner --run "$RUN"
status
cover "$(unit 2)" "$(cand 2)"

say "a record added that references the seed"
writfold kb record decision --title "Decision f" --target-source src/f.rs --references SEED-1 >/dev/null
status
show writfold kb gate run related-scope $(at)
status

say "one member edited"
for f in proposals/decision/*-decision-a/proposal.md; do printf '\nOne sentence added.\n' >> "$f"; done
status
show writfold kb gate run related-scope $(at)
status

say "the seed edited"
for f in ktlo/2026/seed-1-*.md; do printf '\nThe budget is now 5 ms.\n' >> "$f"; done
show writfold kb gate run related-scope $(at)
status

