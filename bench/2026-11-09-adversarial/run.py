"""Adversarial campaign for the trial profile (milestone S4 in
docs/writfold/agent-boundary-plan.md). Drives the real CLI and filesystem; no
fault hooks in production code.

Usage: python3 -I run.py <writfold-binary> <out-dir> [kill-runs=40]

Cases:
  preimage-race       the worktree file changes between approve and apply
  refuted-required    approve --require on a check the report refutes
  unretained-cite     a report cites evidence that was never retained
  tampered-evidence   retained bytes are altered before prepare
  no-change           a recipe whose candidate equals its source
  replay              apply twice; recover after an outcome
  kill-during-apply   SIGKILL apply after a random delay, then recover
  checkout-untouched  the user's checkout is byte-identical at the end
Every case records expected vs observed; results.json keeps failures.
"""
import json
import os
import random
import signal
import subprocess
import sys
import tempfile
import time

BIN, OUT = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
KILLS = int(sys.argv[3]) if len(sys.argv) > 3 else 40
SOURCE = "fn main() {\n    run(1);\n}\n"
CANDIDATE = SOURCE.replace("run(1)", "run(2)")
RECIPE = 'edit "src/main.rs" {\n  @n := one text "run(1)"\n  replace @n with "run(2)"\n}\n'
ENV = dict(os.environ, GIT_AUTHOR_NAME="b", GIT_AUTHOR_EMAIL="b@example.invalid",
           GIT_COMMITTER_NAME="b", GIT_COMMITTER_EMAIL="b@example.invalid")
work = tempfile.mkdtemp(prefix="writfold-adversarial-")
results = []


def sh(*args, cwd=None):
    return subprocess.run(args, cwd=cwd, env=ENV, capture_output=True, text=True)


def wf(*args):
    return sh(BIN, "trial", *args, "--state", STATE)


def record(case, expected, observed, ok, detail=""):
    results.append({"case": case, "expected": expected, "observed": observed, "ok": ok, "detail": detail[:300]})


def report(asset=None, result="entailed"):
    bindings = [{"kind": "asset", "reference": asset}] if asset else []
    return json.dumps({"schema": "weft.report/1", "kind": "assessment", "scope": "local-symbolic",
                       "execution": "complete", "writfold_grounded": False, "effect_authority": False,
                       "review": {"bindings": bindings},
                       "checks": [{"name": "safe", "result": result}]})


def prepare(report_text=None, recipe=RECIPE):
    with open(f"{work}/r.spl", "w") as f:
        f.write(recipe)
    args = ["prepare", "--repo", REPO, "--file", "src/main.rs", "--recipe", f"{work}/r.spl"]
    if report_text is not None:
        with open(f"{work}/report.json", "w") as f:
            f.write(report_text)
        args += ["--report", f"{work}/report.json"]
    p = wf(*args)
    run = next((w for w in p.stdout.split() if w.startswith("run.")), None)
    return p, run


def badge(run, name):
    out = wf("show", run).stdout
    line = next((l for l in out.splitlines() if l.strip().startswith(name)), "")
    return line.strip()[len(name):].strip(), out


def worktree_file(run):
    with open(f"{STATE}/worktrees/{run}/src/main.rs") as f:
        return f.read()


REPO = f"{work}/repo"
STATE = f"{work}/state"
os.makedirs(f"{REPO}/src")
with open(f"{REPO}/src/main.rs", "w") as f:
    f.write(SOURCE)
for a in (["init", "-q"], ["add", "-A"], ["commit", "-qm", "fixture"]):
    sh("git", *a, cwd=REPO)
COMMIT = sh("git", "rev-parse", "HEAD", cwd=REPO).stdout.strip()
checkout_before = sh("git", "status", "--porcelain", "--ignored", cwd=REPO).stdout
wf("init")

# preimage-race
p, run = prepare()
wf("approve", run)
with open(f"{STATE}/worktrees/{run}/src/main.rs", "a") as f:
    f.write("// raced\n")
a = wf("apply", run)
effect, _ = badge(run, "effect")
record("preimage-race", "apply refuses; effect abandoned", f"rc={a.returncode}; effect={effect}",
       a.returncode != 0 and effect.startswith("abandoned"), a.stderr)

# refuted-required
p, run = prepare(report(result="refuted"))
a = wf("approve", run, "--require", "safe")
approval, _ = badge(run, "approval")
record("refuted-required", "approve refuses; approval none", f"rc={a.returncode}; approval={approval}",
       a.returncode != 0 and approval == "none", a.stderr)

# unretained-cite
before = set(os.listdir(f"{STATE}/worktrees"))
p, run = prepare(report(asset="0" * 64))
after = set(os.listdir(f"{STATE}/worktrees"))
record("unretained-cite", "prepare refuses before any worktree", f"rc={p.returncode}; new worktrees={len(after - before)}",
       p.returncode != 0 and after == before, p.stderr)

# tampered-evidence
r = wf("retain", "--repo", REPO, "--commit", COMMIT, "--file", "src/main.rs", "--bytes", "0..11")
binding = r.stdout.split()[2] if r.returncode == 0 else ""
with open(f"{STATE}/evidence/{binding}.bytes", "w") as f:
    f.write("fn evil() {")
p, run = prepare(report(asset=binding))
record("tampered-evidence", "prepare refuses", f"retain rc={r.returncode}; prepare rc={p.returncode}",
       r.returncode == 0 and p.returncode != 0, p.stderr)

# no-change
p, run = prepare(recipe=RECIPE.replace('"run(2)"', '"run(1)"'))
record("no-change", "prepare refuses", f"rc={p.returncode}", p.returncode != 0, p.stderr)

# replay
p, run = prepare()
wf("approve", run)
first, second, rec = wf("apply", run), wf("apply", run), wf("recover", run)
effect, _ = badge(run, "effect")
record("replay", "first applies; second apply and recover refuse; one outcome",
       f"rc={first.returncode},{second.returncode},{rec.returncode}; effect={effect}",
       first.returncode == 0 and second.returncode != 0 and rec.returncode != 0 and effect == "applied")

# kill-during-apply
rng = random.Random(20261109)
tally = {}
for i in range(KILLS):
    p, run = prepare()
    wf("approve", run)
    delay = rng.uniform(0.05, 0.6)
    proc = subprocess.Popen([BIN, "trial", "apply", run, "--state", STATE],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=ENV)
    time.sleep(delay)
    killed = proc.poll() is None
    if killed:
        proc.send_signal(signal.SIGKILL)
    proc.wait()
    effect, shown = badge(run, "effect")
    recovered = None
    if effect.startswith("unknown"):
        recovered = wf("recover", run)
        effect, shown = badge(run, "effect")
    content = worktree_file(run)
    outcomes = sum(1 for l in shown.splitlines() if l.strip()[:3] in ("4. ", "5. "))
    consistent = ((effect.startswith("applied") and content == CANDIDATE)
                  or (effect.startswith("abandoned") and content == SOURCE)
                  or (effect == "none" and content == SOURCE))
    key = f"{'killed' if killed else 'finished'}:{effect.split(' ')[0]}{':recovered' if recovered else ''}"
    tally[key] = tally.get(key, 0) + 1
    if not (consistent and outcomes <= 1):
        record("kill-during-apply", "file matches the single recorded outcome", f"{key}; file={content!r}", False)
record("kill-during-apply", "every interrupted apply ends consistent with at most one outcome",
       json.dumps(tally, sort_keys=True), all(r["ok"] for r in results if r["case"] == "kill-during-apply"))

# checkout-untouched
after = sh("git", "status", "--porcelain", "--ignored", cwd=REPO).stdout
with open(f"{REPO}/src/main.rs") as f:
    same = f.read() == SOURCE
record("checkout-untouched", "user checkout byte-identical, status unchanged", f"same={same}; status_equal={after == checkout_before}",
       same and after == checkout_before)

os.makedirs(OUT, exist_ok=True)
summary = {"binary_sha256": sh("sha256sum", BIN).stdout.split()[0], "kill_runs": KILLS,
           "passed": sum(r["ok"] for r in results), "total": len(results), "results": results}
with open(f"{OUT}/results.json", "w") as f:
    json.dump(summary, f, indent=2)
for r in results:
    print(f"{'ok  ' if r['ok'] else 'FAIL'} {r['case']:20} {r['observed']}")
subprocess.run(["rm", "-rf", work])
sys.exit(0 if summary["passed"] == summary["total"] else 1)
