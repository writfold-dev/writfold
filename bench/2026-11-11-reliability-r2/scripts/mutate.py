#!/usr/bin/env python3
"""Mid-task mutation harness for C06, C07 and C08 (both arms, identical rule).

Adapted from the reliability pilot's T6 harness. Trigger (pre-registered):
the first tool result in the trial agent's own transcript whose tool call
input or result text contains the task's `trigger` (a file name), that is,
the agent's first sight of the file by any tool (grep, Tuft, Read, sed, ...).
As soon as the watcher sees that result it changes the checkout file
`<trial>/repo/<path>` in place, once, as expected.json says (`insert_after`
pinned line n: insert `lines`; `replace_line` n: replace `old` with `new` on
that line), and logs the time and whether the file was still pristine.

Usage:
  mutate.py apply EXPECTED_JSON TASK TRIAL_DIR      (mutate now; reference check)
  mutate.py watch EXPECTED_JSON TASK TRIAL_DIR TRANSCRIPTS_DIR LOG_JSON
The watcher identifies the trial's transcript as the newest *.jsonl in
TRANSCRIPTS_DIR whose first event's prompt contains TRIAL_DIR + '/'. It is
started before the agent is launched and gives up after 40 minutes."""
import datetime, hashlib, json, pathlib, subprocess, sys, time


def mutate(m, trial, commit):
    p = pathlib.Path(trial) / "repo" / m["path"]
    before = p.read_bytes()
    pinned = subprocess.run(["git", "-C", str(pathlib.Path(trial) / "repo"), "show", f"{commit}:{m['path']}"],
                            capture_output=True, check=True).stdout
    lines = before.decode().split("\n")
    applied = True
    if "insert_after" in m:
        n = m["insert_after"]
        lines = lines[:n] + list(m["lines"]) + lines[n:]
    else:
        n = m["replace_line"]
        if m["old"] in lines[n - 1]:
            lines[n - 1] = lines[n - 1].replace(m["old"], m["new"], 1)
        else:
            applied = False
    after = "\n".join(lines).encode()
    if applied:
        with open(p, "r+b") as f:
            f.write(after)
            f.truncate()
    return {"before_sha256": hashlib.sha256(before).hexdigest(), "after_sha256": hashlib.sha256(after).hexdigest(),
            "before_was_pinned": before == pinned, "applied": applied}


def text_of(c):
    if isinstance(c, str):
        return c
    return "".join(x.get("text", "") for x in c if isinstance(x, dict))


def main():
    mode, expected, task, trial = sys.argv[1:5]
    ex = json.load(open(expected))
    m = ex["tasks"][task]["mutation"]
    trial = str(pathlib.Path(trial).resolve())
    if mode == "apply":
        print(json.dumps(mutate(m, trial, ex["commit"])))
        return
    tdir, log = pathlib.Path(sys.argv[5]), sys.argv[6]
    start = time.time()
    marker = trial + "/"
    transcript = None
    while time.time() - start < 2400:
        if transcript is None:
            for p in sorted(tdir.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True):
                if p.stat().st_mtime < start - 5:
                    continue
                try:
                    first = json.loads(p.open().readline())
                except Exception:
                    continue
                if marker in json.dumps(first.get("message", {})):
                    transcript = p
                    break
        if transcript is not None:
            uses = {}
            for line in transcript.read_text(errors="replace").splitlines():
                try:
                    ev = json.loads(line)
                except Exception:
                    continue
                msg = ev.get("message") or {}
                if ev.get("type") == "assistant":
                    for c in msg.get("content", []):
                        if isinstance(c, dict) and c.get("type") == "tool_use":
                            uses[c["id"]] = json.dumps(c.get("input", {}))
                elif ev.get("type") == "user" and isinstance(msg.get("content"), list):
                    for c in msg["content"]:
                        if isinstance(c, dict) and c.get("type") == "tool_result":
                            if m["trigger"] in uses.get(c["tool_use_id"], "") or m["trigger"] in text_of(c.get("content", "")):
                                info = mutate(m, trial, ex["commit"])
                                info.update({"triggered": True, "tool_use_id": c["tool_use_id"],
                                             "result_event_timestamp": ev.get("timestamp"),
                                             "mutated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                                             "transcript": transcript.name})
                                json.dump(info, open(log, "w"), indent=1)
                                return
        time.sleep(0.05)
    json.dump({"triggered": False, "transcript": transcript.name if transcript else None}, open(log, "w"), indent=1)


if __name__ == "__main__":
    main()
