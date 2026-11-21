#!/usr/bin/env python3
# A generated KB shape for capacity measurements (#58, #59): python3 scripts/kb_shape.py OUT N K B.
"""Generate a synthetic KB: N decision records, each referencing K others
by ID in its body (edges), with a body of B filler words. No real data."""
import os, sys, random, subprocess
out, n, k, b = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
random.seed(7)
os.makedirs(f"{out}/inventory/assets", exist_ok=True)
open(f"{out}/inventory/assets/demo.json","w").write('{"id": "demo", "kind": "service", "name": "Demo"}\n')
os.makedirs(f"{out}/harbor", exist_ok=True)
open(f"{out}/harbor/README.md","w").write("# Harbor\n")
d = f"{out}/proposals/decision"
os.makedirs(d, exist_ok=True)
words = ["ledger","flush","tenant","shard","batch","replica","quorum","index","cursor","stall"]
for i in range(n):
    refs = " ".join(f"decision/2026-01-01-d{random.randrange(n):06d}" for _ in range(k))
    body = " ".join(random.choice(words) for _ in range(b))
    open(f"{d}/2026-01-01-d{i:06d}.md","w").write(
        f"---\nid: decision/2026-01-01-d{i:06d}\nkind: decision\ntitle: Decision {i}\nstatus: accepted\ncreated: 2026-01-01\nupdated: 2026-01-01\nproject: harbor\n---\n\n## Context\n\n{body}\n\nSee {refs}.\n")
env = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@x.invalid", GIT_COMMITTER_NAME="t", GIT_COMMITTER_EMAIL="t@x.invalid")
for args in (["init","-q","-b","main"],["add","-A"],["commit","-q","-m","gen"]):
    subprocess.run(["git","-C",out]+args, check=True, env=env)
