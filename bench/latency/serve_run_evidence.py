"""Latency of trial/run, trial/evidence-list and trial/evidence-show through one
`writfold serve` session per repository.

Usage: python3 -I bench/latency/serve_run_evidence.py <writfold-binary> <out-dir> [files=100000] [repeats=10]

Builds the same small and large synthetic repositories as run.sh, retains 1,000
bindings in the large repository's state first (as evidence.sh does), then per
repository negotiates writfold.trial/1 and times, request to response:
trial/run (Tuft, a stub Weft and prepare, as run.sh times `trial run`),
trial/evidence-list (default page, and a path filter) and trial/evidence-show
(full digest, and three context lines from the pinned blob). Weft is a stub;
the real Weft is not timed. Writes samples.tsv and results.json; exits
non-zero if any call fails, exceeds the 5 s ceiling, or a p95 exceeds its target
(1 s for trial/run; 250 ms for the read-only evidence views).
"""
import json
import os
import subprocess
import sys
import tempfile
import time

binary, out = os.path.realpath(sys.argv[1]), os.path.realpath(sys.argv[2])
files = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
repeats = int(sys.argv[4]) if len(sys.argv) > 4 else 10
here = os.path.dirname(os.path.realpath(__file__))
TARGET = {"trial/run": 1000, "evidence-list": 250, "evidence-list-path": 250,
          "evidence-show": 250, "evidence-show-context3": 250}
STUB = """#!/bin/sh
[ "$1 $2 $3" = "prove --json --" ] || exit 9
ref=$(sed -n 's/.*citation "\\([^"]*\\)".*/\\1/p' "$4" | head -n 1)
printf '{"schema":"weft.report/1","kind":"assessment","scope":"local-symbolic","execution":"complete","writfold_grounded":false,"effect_authority":false,"review":{"bindings":[{"kind":"asset","reference":"%s"}]},"checks":[{"name":"ok","result":"entailed"}]}' "$ref"
"""


def write(path, text, mode=0o644):
    with open(path, "w") as f:
        f.write(text)
    os.chmod(path, mode)


def inputs(work, name, head):
    tuft, review = os.path.join(work, f"{name}-run.tuft"), os.path.join(work, f"{name}-review.weft")
    write(tuft, f'@h := find text "open(url)" in "src/" at "{head}";\n@e := retain @h around lines 0;\n'
                '@v := project @e [path, line, binding];\nemit @v;\n')
    write(review, f'review r {{\n  context "trial" at "{head}";\n  profile "weft.symbolic/1";\n'
                  '  target change = proposal "retry";\n  asset call = citation "tuft:@v#1";\n'
                  '  assume a: ok("x");\n  check ok: ok("x");\n}\n')
    return tuft, review


def session(work, state, repo, name, samples):
    global trailers
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo, check=True,
                          capture_output=True, text=True).stdout.strip()
    tuft, review = inputs(work, name, head)
    server = subprocess.Popen([binary, "serve", "--state", state], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, env=dict(os.environ, WRITFOLD_STATE=""))
    ident = 0

    def call(label, method, params):
        nonlocal ident
        ident += 1
        frame = json.dumps({"jsonrpc": "2.0", "id": ident, "method": method, "params": params})
        start = time.perf_counter_ns()
        server.stdin.write(frame.encode() + b"\n")
        server.stdin.flush()
        reply = json.loads(server.stdout.readline())
        elapsed = (time.perf_counter_ns() - start) / 1e6
        if "result" not in reply:
            raise SystemExit(f"{method} refused: {reply}")
        if label in TARGET:
            samples.append((name, label, elapsed))
        return reply["result"].get("payload")

    call("", "initialize", {"protocol_version": "2", "capabilities": ["writfold.trial/1"]})
    for _ in range(repeats):
        ran = call("trial/run", "trial/run", {
            "repo": repo, "file": "src/client.rs", "recipe": os.path.join(work, "edit.spl"),
            "tuft": tuft, "weft": review, "weft_bin": os.path.join(work, "weft")})
        if [c["result"] for c in ran["checks"]] != ["entailed"]:
            raise SystemExit(f"unexpected checks: {ran['checks']}")
        binding = ran["resolved"][0]["binding"]
        trailers[name] = call("evidence-list", "trial/evidence-list", {})["trailer"]
        call("evidence-list-path", "trial/evidence-list", {"path": "src/"})
        call("evidence-show", "trial/evidence-show", {"binding": binding})
        call("evidence-show-context3", "trial/evidence-show",
             {"binding": binding, "context_lines": 3, "repo": repo})
    call("", "shutdown", {})
    if server.wait(30) != 0:
        raise SystemExit("serve did not exit cleanly")


def p95(values):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, round(0.95 * (len(ordered) - 1)))]


with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as work:
    subprocess.run([sys.executable, "-I", os.path.join(here, "repos.py"), work, str(files)], check=True)
    write(os.path.join(work, "weft"), STUB, 0o755)
    samples, trailers = [], {}
    for name in ("small", "large"):
        state, repo = os.path.join(work, f"state-{name}"), os.path.join(work, name)
        subprocess.run([binary, "trial", "init", "--state", state], check=True, capture_output=True)
        if name == "large":
            head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo, check=True,
                                  capture_output=True, text=True).stdout.strip()
            program = os.path.join(work, "retain.tuft")
            write(program, f'@h := find rx"^fn f[0-9]+\\(\\)" at "{head}" limit 1000;\n'
                           '@e := retain @h around lines 0;\n@v := project @e [path, line];\n'
                           'emit @v budget_tokens 100000;\n')
            subprocess.run([binary, "tuft", "run", program, "--repo", repo, "--state", state],
                           check=True, capture_output=True)
        session(work, state, repo, name, samples)
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, "samples.tsv"), "w") as f:
    f.writelines(f"{repo}\t{op}\t{ms:.1f}\n" for repo, op, ms in samples)
operations, failed = [], False
for repo in ("small", "large"):
    for op, target in TARGET.items():
        values = [ms for r, o, ms in samples if r == repo and o == op]
        ok = max(values) <= 5000 and p95(values) <= target
        failed |= not ok
        operations.append(dict(repo=repo, op=op, samples=len(values),
                               median_ms=round(sorted(values)[len(values) // 2], 1),
                               p95_ms=round(p95(values), 1), max_ms=round(max(values), 1),
                               target_p95_ms=target, within_budget=ok))
with open(os.path.join(out, "results.json"), "w") as f:
    json.dump(dict(transport="writfold serve (stdio JSON-RPC), one session per repository",
                   weft="stub (fixed weft.report/1); the real Weft is not timed",
                   ceiling_ms=5000, large_files=files, large_retained_before=1000,
                   repeats=repeats, last_list_trailer=trailers,
                   operations=operations), f, indent=2)
    f.write("\n")
for row in operations:
    print(f"{row['repo']:5s} {row['op']:22s} median {row['median_ms']:6.1f} ms  p95 {row['p95_ms']:6.1f} ms"
          f"  max {row['max_ms']:6.1f} ms  {'ok' if row['within_budget'] else 'OVER'}")
sys.exit(1 if failed else 0)
