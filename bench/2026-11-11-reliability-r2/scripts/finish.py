#!/usr/bin/env python3
"""Operational helper (added after pre-registration; grades nothing): after a
trial's agent finishes, run collect.sh on its transcript, append its usage
row, and check that the transcript's first prompt equals the trial's
.prompt file byte for byte.

Usage: finish.py SCRATCH TRANSCRIPTS_DIR USAGE_TXT TRIAL AGENT_ID TOKENS TOOL_USES DURATION_MS"""
import json, pathlib, subprocess, sys

S, TD, U = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
trial, aid, tok, tools, ms = sys.argv[4:9]
tr = TD / f"agent-{aid}.jsonl"
first = json.loads(tr.open().readline())
content = first["message"]["content"]
if isinstance(content, list):
    content = "".join(c.get("text", "") for c in content if isinstance(c, dict))
want = (S / "trials" / f"{trial}.prompt").read_text()
same = content == want or content == want.rstrip("\n")
out = subprocess.run(["bash", str(pathlib.Path(__file__).parent / "collect.sh"), str(S), trial, str(tr)],
                     capture_output=True, text=True, check=True).stdout.strip()
with U.open("a") as f:
    f.write(f"{trial} {tok} {tools} {ms}\n")
log = S / "prompt-check.txt"
with log.open("a") as f:
    f.write(f"{trial} {'equal' if same else 'DIFFERENT'}\n")
print(out, "prompt", "equal" if same else "DIFFERENT")
