#!/usr/bin/env python3
"""T6 mid-session mutation harness (both arms, identical rule).

Trigger (pre-registered): the first tool result in the trial agent's own
transcript whose tool call input or result text contains the target path
`writfold-wire/src/rpc.rs`. That is the agent's first sight of the target
file, by any tool (grep, Tuft, Read, sed, ...). As soon as the watcher sees
that result it inserts the two lines in expected.json `edits.T6.mutation_lines`
after line 5 of the working file `<trial>/repo/crates/writfold-wire/src/rpc.rs`
(in place, whatever its content then is), once, and logs the time. The log
also records the transcript timestamp of the agent's next tool call, so the
grader can check that the mutation landed before the agent acted again.

Usage:
  mutate.py apply EXPECTED_JSON TRIAL_DIR            (mutate now; reference check)
  mutate.py watch EXPECTED_JSON TRIAL_DIR TRANSCRIPTS_DIR LOG_JSON
The watcher identifies the trial's transcript as the newest *.jsonl in
TRANSCRIPTS_DIR whose first event's prompt contains TRIAL_DIR + '/'. It is
started before the agent is launched and gives up after 40 minutes."""
import datetime, hashlib, json, pathlib, sys, time


def mutate(e, trial):
    p = pathlib.Path(trial) / "repo" / e["path"]
    before = p.read_bytes()
    lines = before.decode().split("\n")
    n = e["insert_after_line"]
    after = "\n".join(lines[:n] + e["mutation_lines"] + lines[n:]).encode()
    with open(p, "r+b") as f:
        f.write(after)
        f.truncate()
    return {"before_sha256": hashlib.sha256(before).hexdigest(), "after_sha256": hashlib.sha256(after).hexdigest(),
            "before_was_pinned": before.decode() == e["mutated"].replace("\n".join(e["mutation_lines"]) + "\n", "", 1)}


def text_of(c):
    if isinstance(c, str):
        return c
    return "".join(x.get("text", "") for x in c if isinstance(x, dict))


def main():
    mode, expected, trial = sys.argv[1:4]
    e = json.load(open(expected))["edits"]["T6"]
    trial = str(pathlib.Path(trial).resolve())
    if mode == "apply":
        print(json.dumps(mutate(e, trial)))
        return
    tdir, log = pathlib.Path(sys.argv[4]), sys.argv[5]
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
                m = ev.get("message") or {}
                if ev.get("type") == "assistant":
                    for c in m.get("content", []):
                        if isinstance(c, dict) and c.get("type") == "tool_use":
                            uses[c["id"]] = json.dumps(c.get("input", {}))
                elif ev.get("type") == "user" and isinstance(m.get("content"), list):
                    for c in m["content"]:
                        if isinstance(c, dict) and c.get("type") == "tool_result":
                            if e["trigger"] in uses.get(c["tool_use_id"], "") or e["trigger"] in text_of(c.get("content", "")):
                                info = mutate(e, trial)
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
