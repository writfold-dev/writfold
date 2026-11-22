#!/bin/sh
# A cutover rehearsal (docs/guide/validation.md, "Rehearsing a cutover";
# #96): one change, an author and a receiver, two clones each. The author
# records and accepts a decision, works in a run, runs the gates its plan
# owes (skipping one, and weakening another's definition in the candidate),
# and pushes. The receiver judges the author's evidence with `handoff
# check` (refused), runs every required gate itself with the policy
# revision's definitions into its own run (accepted), lands the candidate
# and moves the decision to applied and verified.
#
#   cutover-rehearsal.sh DIR      DIR must not exist; everything is made there
#
# Needs `writfold` and `git` on PATH. Nothing outside DIR is read or written.
set -eu
DIR=$1
[ ! -e "$DIR" ] || { echo "refusing: $DIR exists" >&2; exit 1; }
mkdir -p "$DIR" && DIR=$(cd "$DIR" && pwd)
export GIT_AUTHOR_NAME=rehearsal GIT_AUTHOR_EMAIL=rehearsal@example.invalid
export GIT_COMMITTER_NAME=rehearsal GIT_COMMITTER_EMAIL=rehearsal@example.invalid
unset WRITFOLD_KB WRITFOLD_SESSION
say() { printf '\n## %s\n' "$*"; }
show() { printf '$ %s\n' "$*"; "$@" || true; }
gate() {
  printf 'schema = "writfold-gate/1"\nid = "%s"\nversion = 1\nclass = "mechanical"\nargv = ["sh", "-c", "%s"]\ntimeout_seconds = 30\nscope = ["src/**"]\n' "$1" "$2"
}

say "origin: the service with its policy, and a shared KB"
git init -q --bare -b main "$DIR/origin/service.git"
git init -q --bare -b main "$DIR/origin/kb.git"
git clone -q "$DIR/origin/service.git" "$DIR/seed" 2>/dev/null
cd "$DIR/seed" && git checkout -q -b main
mkdir -p src .writfold/gates
echo "status: ok" > src/app.txt
gate build 'test -f src/app.txt' > .writfold/gates/build.toml
gate test 'grep -q ok src/app.txt' > .writfold/gates/test.toml
gate lint '! grep -q TODO src/app.txt' > .writfold/gates/lint.toml
printf 'schema = "writfold-handoff/1"\nrequired = ["build", "test", "lint"]\n' > .writfold/handoff.toml
git add -A && git commit -qm "service with its gates and handoff policy" && git push -q origin main
POLICY=$(git rev-parse HEAD)
git clone -q "$DIR/origin/kb.git" "$DIR/kbseed" 2>/dev/null
cd "$DIR/kbseed" && git checkout -q -b main && writfold kb init >/dev/null
mkdir -p .writfold/plans
printf 'schema = "writfold-plan/1"\nname = "ship"\nsteps = ["build: build", "test: test", "lint: lint"]\n' > .writfold/plans/ship.toml
git add -A && git commit -qm "KB with the ship plan" && git push -q origin main
echo "policy revision: $POLICY"

say "author: decide, work in a run, run the gates the plan owes"
git clone -q "$DIR/origin/service.git" "$DIR/author/service"
git clone -q "$DIR/origin/kb.git" "$DIR/author/kb"
cd "$DIR/author/kb"
mkdir -p .git/writfold && printf '# local repository map\nservice %s\n' "$DIR/author/service" > .git/writfold/repos
writfold kb propose decision --project service --target-source service \
  --title "Report status in app.txt" --context "status is unreported" \
  --change "write a status line" --why "the receiver checks it" --session author
ID=$(writfold kb proposals --json | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | head -1)
show writfold kb proposals accept "$ID"
DECISION=$(writfold kb find "Report status" --json | sed -n 's/.*"id":"\(proposal\/decision[^"]*\)".*/\1/p' | head -1)
FILE=$(ls proposals/decision/*/proposal.md)
awk '{ print }
  /^## Research$/ { print "\nThe receiver'"'"'s test gate reads the status line." }
  /^## Alternatives considered$/ { print "\n- A log line: no gate reads it.\n- No change: the status stays unreported." }
  /^## Acceptance gates$/ { print "\n| id | when | claim | check | by |\n|---|---|---|---|---|\n| test | before landing | the status line reads ok | gate test | receiver |" }' \
  "$FILE" > "$FILE.new" && mv "$FILE.new" "$FILE"
show writfold kb status "$DECISION" --set proposed
show writfold kb status "$DECISION" --set accepted --by owner
git add -A && git commit -qm "decision accepted" && git push -q origin main
cd "$DIR/author/service" && git checkout -q -b change
echo "status: ok, reported" > src/app.txt
git add -A && git commit -qm "report status" && git push -q origin change
CANDIDATE=$(git rev-parse HEAD)
gate test 'true' > .writfold/gates/test.toml   # weakened locally, never committed
export WRITFOLD_KB="$DIR/author/kb"
writfold kb checkpoint --new-run --project service --task "report status" --note "started" >/dev/null
writfold kb plan set ship >/dev/null
writfold kb gate run build >/dev/null && writfold kb plan advance >/dev/null
writfold kb gate run test >/dev/null && writfold kb plan advance >/dev/null
writfold kb plan skip --reason "lint is slow; later" --by author >/dev/null
AUTHOR_RUN=$(writfold kb runs --json | sed -n 's/.*"id":"\(run-[^"]*\)".*/\1/p' | head -1)
show writfold kb gate lines
git checkout -q -- .writfold
cd "$DIR/author/kb" && git add -A && git commit -qm "author's run" && git push -q origin main
unset WRITFOLD_KB

say "receiver: judge the author's evidence (refused)"
git clone -q "$DIR/origin/service.git" "$DIR/receiver/service"
git clone -q "$DIR/origin/kb.git" "$DIR/receiver/kb"
cd "$DIR/receiver/kb"
mkdir -p .git/writfold && printf '# local repository map\nservice %s\n' "$DIR/receiver/service" > .git/writfold/repos
export WRITFOLD_KB="$DIR/receiver/kb"
cd "$DIR/receiver/service" && git checkout -q "$CANDIDATE"
show writfold kb handoff check --policy "$POLICY" --candidate "$CANDIDATE" --run "$AUTHOR_RUN"

say "receiver: run every required gate with the policy's definitions, into its own run"
git checkout -q "$POLICY" -- .writfold    # the trusted definitions, over the candidate's
writfold kb checkpoint --new-run --project service --task "receive $CANDIDATE" --note "receiving" >/dev/null
for g in build test lint; do writfold kb gate run "$g" >/dev/null || true; done
show writfold kb handoff check --policy "$POLICY" --candidate "$CANDIDATE"

say "landing: merge, then applied and verified"
git checkout -q main && git merge -q --ff-only "$CANDIDATE" && git push -q origin main
show writfold kb status "$DECISION" --set applied --commit "service@$CANDIDATE"
show writfold kb status "$DECISION" --set verified --evidence gate:test
cd "$DIR/receiver/kb" && git add -A && git commit -qm "receiver's run; decision verified" && git push -q origin main
