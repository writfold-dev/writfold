#!/bin/sh
# Latency bench for agent-facing trial operations (docs/writfold/agent-boundary-plan.md).
# Usage: bench/latency/run.sh <writfold-binary> <out-dir> [files=100000] [repeats=5]
# Builds a small and a large synthetic repository, runs prepare/approve/apply/show
# and three `tuft run` programs
# `repeats` times on each, then `trial run` with a stub Weft and its parts,
# and writes results.json. Exits non-zero if any single
# operation exceeds the 5 s ceiling or a p95 exceeds the 1 s target.
set -eu
bin=$(realpath "$1"); out=$(realpath -m "$2"); files=${3:-100000}; repeats=${4:-5}
here=$(dirname "$(realpath "$0")")
mkdir -p "$out"; work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
python3 -I "$here/repos.py" "$work" "$files"
export WRITFOLD_STATE="$work/state"
"$bin" trial init >/dev/null
ms() { s=$(date +%s%N); "$@" >"$work/last" 2>&1 || { cat "$work/last" >&2; exit 1; }; echo $(( ($(date +%s%N) - s) / 1000000 )); }
: >"$out/samples.tsv"
for repo in small large; do
  i=0
  while [ "$i" -lt "$repeats" ]; do
    t=$(ms "$bin" trial prepare --repo "$work/$repo" --file src/client.rs --recipe "$work/edit.spl")
    run=$(grep -o 'run\.[0-9a-f.]*' "$work/last" | head -1)
    printf '%s\tprepare\t%s\n' "$repo" "$t" >>"$out/samples.tsv"
    for op in approve apply show; do
      printf '%s\t%s\t%s\n' "$repo" "$op" "$(ms "$bin" trial "$op" "$run")" >>"$out/samples.tsv"
    done
    i=$((i + 1))
  done
done
# Tuft v0 (milestone T0) over the whole pinned tree: exact text with retained
# whole-line excerpts, a regex over every file, and a retain-heavy run that
# retains 50 hits in 50 files.
for repo in small large; do
  c=$(git -C "$work/$repo" rev-parse HEAD)
  printf '@h := find text "open(url)" at "%s" limit 50;\n@e := retain @h around lines 1;\n@v := project @e [path, line, binding, excerpt(max_chars: 120)] limit 6;\nemit @v budget_tokens 400;\n' "$c" >"$work/$repo-text.tuft"
  printf '@h := find rx"open\\(\\w+\\)" at "%s" limit 50;\n@v := project @h [path, line, range];\nemit @v budget_tokens 400;\n' "$c" >"$work/$repo-rx.tuft"
  printf '@h := find text "fn " at "%s" limit 50;\n@e := retain @h;\n@v := project @e [path, line, excerpt(max_chars: 40)];\nemit @v budget_tokens 400;\n' "$c" >"$work/$repo-retain50.tuft"
  i=0
  while [ "$i" -lt "$repeats" ]; do
    for q in text rx retain50; do
      printf '%s\ttuft-%s\t%s\n' "$repo" "$q" "$(ms "$bin" tuft run "$work/$repo-$q.tuft" --repo "$work/$repo")" >>"$out/samples.tsv"
    done
    i=$((i + 1))
  done
done
# `trial run` (milestone T1) with a stub Weft that emits a fixed report citing
# the source's first `citation`, against its parts run one by one: `tuft run`,
# the stub on the same substituted source, and `trial prepare --report`.
cat >"$work/weft" <<'EOF'
#!/bin/sh
ref=$(sed -n 's/.*citation "\([^"]*\)".*/\1/p' "$4" | head -n 1)
printf '{"schema":"weft.report/1","kind":"assessment","scope":"local-symbolic","execution":"complete","writfold_grounded":false,"effect_authority":false,"review":{"bindings":[{"kind":"asset","reference":"%s"}]},"checks":[{"name":"ok","result":"entailed"}]}' "$ref"
EOF
chmod 755 "$work/weft"
for repo in small large; do
  c=$(git -C "$work/$repo" rev-parse HEAD)
  printf '@h := find text "open(url)" in "src/" at "%s";\n@e := retain @h around lines 0;\n@v := project @e [path, line, binding];\nemit @v;\n' "$c" >"$work/$repo-run.tuft"
  printf 'review r {\n  context "trial" at "%s";\n  profile "weft.symbolic/1";\n  target change = proposal "retry";\n  asset call = citation "tuft:@v#1";\n  assume a: ok("x");\n  check ok: ok("x");\n}\n' "$c" >"$work/$repo-review.weft"
  i=0
  while [ "$i" -lt "$repeats" ]; do
    printf '%s\ttrial-run\t%s\n' "$repo" "$(ms "$bin" trial run --repo "$work/$repo" --file src/client.rs --recipe "$work/edit.spl" --tuft "$work/$repo-run.tuft" --weft "$work/$repo-review.weft" --weft-bin "$work/weft")" >>"$out/samples.tsv"
    substituted=$(ls -t "$WRITFOLD_STATE"/weft/*.weft | head -n 1)
    printf '%s\tpart-tuft\t%s\n' "$repo" "$(ms "$bin" tuft run "$work/$repo-run.tuft" --repo "$work/$repo")" >>"$out/samples.tsv"
    printf '%s\tpart-weft\t%s\n' "$repo" "$(ms "$work/weft" prove --json -- "$substituted")" >>"$out/samples.tsv"
    cp "$work/last" "$work/report.json"
    printf '%s\tpart-prepare\t%s\n' "$repo" "$(ms "$bin" trial prepare --repo "$work/$repo" --file src/client.rs --recipe "$work/edit.spl" --report "$work/report.json")" >>"$out/samples.tsv"
    i=$((i + 1))
  done
done
python3 -I "$here/summary.py" "$out/samples.tsv" "$out/results.json" "$files"
