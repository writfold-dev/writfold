#!/bin/sh
# Rehearse the optional Plyrune gate (#106) end to end against a stand-in
# checker: a service repository with a requirement and the plyrune-check.sh
# gate, a KB with one run, and the cases the integration must get right.
# Each case prints a "## " section with what `gate run` and `gate status`
# said.
#
#   plyrune-rehearsal.sh DIR STANDIN_BIN    DIR must not exist; STANDIN_BIN is
#                                           the directory holding `plyrune`
#                                           (fixtures/plyrune/bin for the
#                                           stand-in, or a pinned real one)
#
# Needs `writfold` and `git` on PATH. Nothing outside DIR is written.
set -eu
DIR=$1
STANDIN=$(cd "$2" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
[ ! -e "$DIR" ] || { echo "refusing: $DIR exists" >&2; exit 1; }
mkdir -p "$DIR" && DIR=$(cd "$DIR" && pwd)
export GIT_AUTHOR_NAME=rehearsal GIT_AUTHOR_EMAIL=rehearsal@example.invalid
export GIT_COMMITTER_NAME=rehearsal GIT_COMMITTER_EMAIL=rehearsal@example.invalid
unset WRITFOLD_KB WRITFOLD_SESSION
BASE_PATH=$PATH
say() { printf '\n## %s\n' "$*"; }
show() { printf '$ %s\n' "$*"; "$@" || true; }
with_checker() { PATH="$STANDIN:$BASE_PATH"; export PATH; }
without_checker() { PATH="$BASE_PATH"; export PATH; }
run() { show writfold kb gate run plyrune --run "$RUN" --repo "$DIR/service"; }
status() { show writfold kb gate status --run "$RUN" --repo "$DIR/service"; }

say "setup: a service with a requirement and the gate, a KB with a run"
git init -q -b main "$DIR/service"
cd "$DIR/service"
mkdir -p src requirements .writfold/gates
echo "fn main() {}" > src/main.rs
echo "The service starts." > requirements/r1.md
cp "$HERE/plyrune-check.sh" .writfold/gates/plyrune-check.sh
cat > .writfold/gates/plyrune.toml <<'TOML'
schema = "writfold-gate/1"
id = "plyrune"
version = 1
class = "mechanical"
argv = ["sh", "{gate}/plyrune-check.sh", "{artifacts}", "requirements/r1.md"]
timeout_seconds = 60
scope = ["src/**", "requirements/**"]
retain = ["report.json"]
TOML
git add -A && git commit -qm "service with its requirement and Plyrune gate"
git init -q -b main "$DIR/kb"
cd "$DIR/kb" && writfold kb init >/dev/null
writfold kb checkpoint --new-run --project service --task "Plyrune rehearsal" --note "started" >/dev/null
RUN=$(ls runs | head -1)
git add -A && git commit -qm "KB with a run"

say "no checker installed"
without_checker
run

say "checker at another version"
with_checker
PLYRUNE_STANDIN_VERSION=9.9.9 run

say "checker says no"
PLYRUNE_STANDIN_MODE=fail run

say "checker exits 0 but writes no report"
PLYRUNE_STANDIN_MODE=noreport run

say "checker crashes mid-report"
PLYRUNE_STANDIN_MODE=crash run

say "checker satisfied"
PLYRUNE_STANDIN_MODE=pass run
status
show ls runs/"$RUN"/evidence

say "requirement edited"
cd "$DIR/service" && echo "The service starts quickly." > requirements/r1.md && cd "$DIR/kb"
status
cd "$DIR/service" && git checkout -q requirements/r1.md && cd "$DIR/kb"

say "new in-scope file"
cd "$DIR/service" && echo "fn helper() {}" > src/extra.rs && cd "$DIR/kb"
status
rm "$DIR/service/src/extra.rs"

say "fresh clone with the evidence"
git add -A && git commit -qm "run with its Plyrune receipt and evidence"
git clone -q "$DIR/kb" "$DIR/clone"
cd "$DIR/clone"
status

say "fresh clone without the evidence"
rm -r runs/*/evidence
status
