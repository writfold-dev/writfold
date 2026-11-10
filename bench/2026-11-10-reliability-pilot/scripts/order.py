#!/usr/bin/env python3
"""Write order.json: 8 tasks x 2 arms shuffled within each of 3 repeat blocks."""
import json, pathlib, random
root = pathlib.Path(__file__).resolve().parents[1]
SEED = 20261112
rng = random.Random(SEED)
order = []
for r in (1, 2, 3):
    block = [f"T{t}-{a}-r{r}" for t in range(1, 9) for a in "AB"]
    rng.shuffle(block)
    order += block
(root / "order.json").write_text(json.dumps({
    "seed": SEED,
    "method": "random.Random(seed).shuffle per repeat block; trials launched three at a time in this order, each next trial launched as soon as a running one finishes",
    "order": order}, indent=1) + "\n")
print(" ".join(order))
