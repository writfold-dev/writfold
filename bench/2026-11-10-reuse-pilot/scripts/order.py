#!/usr/bin/env python3
"""Write order.json: session-2 trials shuffled per repeat block."""
import json, pathlib, random
root = pathlib.Path(__file__).resolve().parents[1]
SEED = 20261111
rng = random.Random(SEED)
order = []
for r in (1, 2):
    block = [f"R{t}-{a}-r{r}" for t in range(1, 7) for a in "AB"]
    rng.shuffle(block)
    order += block
(root / "order.json").write_text(json.dumps({
    "seed": SEED,
    "method": "random.Random(seed).shuffle per repeat block; session 2 launched three at a time in this order, after all six session-1 runs",
    "session1": [f"S1-R{t}" for t in range(1, 7)], "order": order}, indent=1) + "\n")
print(" ".join(order))
