#!/usr/bin/env bash
# Positive control, run before any trial: the arm-B edit flow in the prompt
# (Splicery recipe -> trial prepare/approve/apply -> git diff | git apply)
# reaches an accepted final state for T5, and for T6 with the mutation
# applied after the read, exactly as the harness applies it.
# Usage: check_reference.sh SCRATCH TEMPLATE
set -euo pipefail
S=$1 T=$2
here=$(cd "$(dirname "$0")/.." && pwd)
W=$S/bin/writfold
run_case() {  # task file old new
  local task=$1 file=$2 old=$3 new=$4 d=$S/reference/$1
  rm -rf "$d"; mkdir -p "$d/work"
  git -c advice.detachedHead=false clone -q "$T" "$d/repo"; git -C "$d/repo" remote remove origin
  "$W" trial init --state "$d/state" >/dev/null
  if [ "$task" = T6 ]; then python3 "$here/scripts/mutate.py" apply "$here/expected.json" "$d"; fi
  printf 'edit "%s" {\n  @x := one text "%s"\n  replace @x with "%s"\n}\n' "$file" "$old" "$new" > "$d/work/e.spl"
  export WRITFOLD_STATE=$d/state
  run=$("$W" trial prepare --repo "$d/repo" --file "$file" --recipe "$d/work/e.spl" | sed -n 's/^prepared run //p')
  "$W" trial approve "$run" >/dev/null
  "$W" trial apply "$run" >/dev/null
  (cd "$d/state/worktrees/$run" && git diff > "$d/work/e.patch")
  (cd "$d/repo" && git apply "$d/work/e.patch")
  python3 - "$here/expected.json" "$task" "$d/repo" <<'EOF'
import json, sys
e = json.load(open(sys.argv[1]))["edits"][sys.argv[2]]
got = open(f"{sys.argv[3]}/{e['path']}").read()
ok = got in e["accepted"]
print(sys.argv[2], "accepted" if ok else "NOT ACCEPTED")
sys.exit(0 if ok else 1)
EOF
}
run_case T5 crates/tuft/src/ir.rs "MAX_SCOPE_BYTES: usize = 1_024;" "MAX_SCOPE_BYTES: usize = 2_048;"
run_case T6 crates/writfold-wire/src/rpc.rs "MAX_REQUEST_BYTES: usize = 65_536;" "MAX_REQUEST_BYTES: usize = 131_072;"
