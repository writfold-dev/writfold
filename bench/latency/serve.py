"""Latency of the writfold.trial/1 RPCs through one `writfold serve` session.

Usage: python3 -I bench/latency/serve.py <writfold-binary> <out-dir> [files=100000] [repeats=10]

Builds the same small and large synthetic repositories as run.sh, starts one
`writfold serve --state` per repository, negotiates writfold.trial/1 and times
trial/retain, trial/prepare and trial/show request to response. Writes
samples.tsv and results.json; exits non-zero if any call fails, exceeds the 5 s
ceiling, or a p95 exceeds its target (1 s; 250 ms for trial/show).
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
TARGET = {"trial/retain": 1000, "trial/prepare": 1000, "trial/show": 250}


def session(state, repo, recipe, samples, name):
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo, check=True,
                          capture_output=True, text=True).stdout.strip()
    server = subprocess.Popen([binary, "serve", "--state", state], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, env=dict(os.environ, WRITFOLD_STATE=""))
    ident = 0

    def call(method, params):
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
        if method in TARGET:
            samples.append((name, method, elapsed))
        return reply["result"]

    call("initialize", {"protocol_version": "2", "capabilities": ["writfold.trial/1"]})
    for _ in range(repeats):
        call("trial/retain", {"repo": repo, "commit": head, "file": "src/client.rs", "start": 0, "end": 40})
        run = call("trial/prepare", {"repo": repo, "file": "src/client.rs", "recipe": recipe})["payload"]["run"]
        call("trial/show", {"run": run})
    call("shutdown", {})
    if server.wait(30) != 0:
        raise SystemExit("serve did not exit cleanly")


def p95(values):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, round(0.95 * (len(ordered) - 1)))]


with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as work:
    subprocess.run([sys.executable, "-I", os.path.join(here, "repos.py"), work, str(files)], check=True)
    samples = []
    for name in ("small", "large"):
        state = os.path.join(work, f"state-{name}")
        subprocess.run([binary, "trial", "init", "--state", state], check=True, capture_output=True)
        session(state, os.path.join(work, name), os.path.join(work, "edit.spl"), samples, name)
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
                   ceiling_ms=5000, large_files=files, repeats=repeats, operations=operations), f, indent=2)
    f.write("\n")
for row in operations:
    print(f"{row['repo']:5s} {row['op']:13s} median {row['median_ms']:6.1f} ms  p95 {row['p95_ms']:6.1f} ms"
          f"  max {row['max_ms']:6.1f} ms  {'ok' if row['within_budget'] else 'OVER'}")
sys.exit(1 if failed else 0)
