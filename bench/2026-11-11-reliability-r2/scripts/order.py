#!/usr/bin/env python3
"""Write order.json.

  order.py calibration          12 candidates x arm A x 2 repeats, shuffled
                                within each repeat block, seed 20261111.
  order.py main C.. C.. ...     the selected tasks x 2 arms x 3 repeats,
                                shuffled within each repeat block, seed
                                20261113 (the list is sorted first, so the
                                order depends only on which tasks qualified).
Trial ids: <K|M>-<task>-<arm>-r<n>."""
import json, pathlib, random, sys

root = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / "scripts"))
from tasks import TASKS  # noqa: E402

path = root / "order.json"
data = json.loads(path.read_text()) if path.exists() else {}
stage = sys.argv[1]
if stage == "calibration":
    seed, tasks, arms, reps, tag = 20261111, sorted(TASKS), "A", (1, 2), "K"
else:
    seed, tasks, arms, reps, tag = 20261113, sorted(sys.argv[2:]), "AB", (1, 2, 3), "M"
    assert 6 <= len(tasks) <= 8 and all(t in TASKS for t in tasks)
rng = random.Random(seed)
order = []
for r in reps:
    block = [f"{tag}-{t}-{a}-r{r}" for t in tasks for a in arms]
    rng.shuffle(block)
    order += block
data[stage] = {"seed": seed, "tasks": tasks,
               "method": "random.Random(seed).shuffle per repeat block; trials launched at most three at a time in this order, each next trial launched as soon as a running one finishes",
               "order": order}
path.write_text(json.dumps(data, indent=1) + "\n")
print(" ".join(order))
