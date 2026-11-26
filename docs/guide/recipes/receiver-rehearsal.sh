#!/bin/sh
# Rehearse the receiver's pre-receive hook (#64, recipes/receiver/pre-receive)
# on a bare repository: a push whose candidate passes the required gate
# lands; pushes that weaken the gate, drop it from the required set, carry
# a forged receipt, or skip it in a plan are refused, because the receiver
# runs the policy revision's gates itself, into its own KB. A push to
# another branch is not judged. Each case prints a "## " section.
#
#   receiver-rehearsal.sh DIR     DIR must not exist
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
gate() {
    printf 'schema = "writfold-gate/1"\nid = "%s"\nversion = 1\nclass = "mechanical"\nargv = ["sh", "-c", "%s"]\ntimeout_seconds = 30\nscope = ["src/**"]\n' "$1" "$2"
}
push() { git push -q origin "HEAD:${1:-main}" 2>&1 | sed -n 's/^remote: //p' | grep -E "^handoff|pre-receive|^gate|^# coverage" || true; printf 'landed=%s\n' "$(git ls-remote origin refs/heads/main | cut -c1-12)"; }

say "setup: a service with a test gate and its handoff policy, and the receiver's KB"
git init -q --bare "$DIR/service.git"
git clone -q "$DIR/service.git" "$DIR/author" 2>/dev/null
cd "$DIR/author"
mkdir -p src .writfold/gates
echo "status: ok" > src/app.txt
gate test 'grep -q ok src/app.txt' > .writfold/gates/test.toml
printf 'schema = "writfold-handoff/1"\nrequired = ["test"]\n' > .writfold/handoff.toml
git add -A && git commit -qm "service, its test gate and the handoff policy"
git branch -M main && git push -q origin main
git init -q -b main "$DIR/receiver-kb"
(cd "$DIR/receiver-kb" && writfold kb init >/dev/null && git add -A && git commit -qm "receiver KB")
printf '#!/bin/sh\nexport WRITFOLD_RECEIVER_KB=%s WRITFOLD_PROJECT=service\nexec sh %s\n' \
    "$DIR/receiver-kb" "$HERE/receiver/pre-receive" > "$DIR/service.git/hooks/pre-receive"
chmod +x "$DIR/service.git/hooks/pre-receive"
POLICY=$(git rev-parse HEAD)

say "a candidate that passes"
echo "status: ok, reported" > src/app.txt
git commit -qam "report status"
push

say "a candidate that fails and weakens the gate"
echo "status: failing" > src/app.txt
gate test 'true' > .writfold/gates/test.toml
git commit -qam "break it, and weaken the test"
push
git reset -q --hard origin/main

say "a candidate that drops the gate from the required set"
echo "status: failing" > src/app.txt
printf 'schema = "writfold-handoff/1"\nrequired = []\n' > .writfold/handoff.toml
git commit -qam "break it, and require nothing"
push
git reset -q --hard origin/main

say "a candidate that carries a forged receipt and a plan that skips the gate"
echo "status: failing" > src/app.txt
mkdir -p runs/forged/gates .writfold/plans
printf '{"gate": "test", "result": {"outcome": "pass"}}\n' > runs/forged/gates/test-forged.json
printf 'schema = "writfold-plan/1"\nname = "ship"\nsteps = ["build:"]\n' > .writfold/plans/ship.toml
git add -A && git commit -qm "break it, forge a pass, plan around the gate"
push
git reset -q --hard origin/main

say "a push to another branch"
echo "status: failing" > src/app.txt
git commit -qam "work in progress"
git push -q origin HEAD:wip 2>&1 | sed -n 's/^remote: //p' || true
printf 'wip=%s main=%s\n' "$(git ls-remote origin refs/heads/wip | cut -c1-12)" "$(git ls-remote origin refs/heads/main | cut -c1-12)"
