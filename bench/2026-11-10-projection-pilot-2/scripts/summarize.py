#!/usr/bin/env python3
"""Summarize each trial's subagent transcript: model, per-call usage, tool calls,
bytes of tool output returned to the model. Paths are replaced by placeholders.
Usage: summarize.py AGENTS_TXT TASKS_DIR SCRATCH OUT_DIR"""
import json, pathlib, sys
HOME = str(pathlib.Path.home())

agents, tasks_dir, scratch, out = sys.argv[1:5]
out = pathlib.Path(out)
out.mkdir(parents=True, exist_ok=True)


def scrub(text, trial):
    t = f"{scratch}/trials/{trial}"
    return (text.replace(t, "<trial>").replace(f"{scratch}/bin/writfold", "<writfold>")
            .replace(scratch, "<scratch>").replace(HOME, "<home>"))


def text_of(content):
    if isinstance(content, str):
        return content
    return "".join(c.get("text", "") for c in content if isinstance(c, dict))


for row in pathlib.Path(agents).read_text().split("\n"):
    if not row.strip():
        continue
    trial, agent, batch = row.split()
    events = [json.loads(l) for l in open(f"{tasks_dir}/{agent}.output")]
    models, calls, tools, results = set(), [], [], {}
    seen_msg = set()
    for e in events:
        m = e.get("message") or {}
        if e.get("type") == "assistant":
            models.add(m.get("model"))
            mid = m.get("id")
            if mid not in seen_msg and m.get("usage"):
                seen_msg.add(mid)
                u = m["usage"]
                calls.append({k: u.get(k, 0) for k in ("input_tokens", "cache_creation_input_tokens",
                                                       "cache_read_input_tokens", "output_tokens")})
            for c in m.get("content", []):
                if c.get("type") == "tool_use":
                    inp = c.get("input", {})
                    shown = inp.get("command") or inp.get("file_path") or json.dumps(inp)[:400]
                    if c["name"] == "Write":
                        shown = f"{inp.get('file_path')} <= {inp.get('content', '')}"
                    tools.append({"id": c["id"], "tool": c["name"], "input": scrub(str(shown), trial)[:2000]})
        elif e.get("type") == "user" and isinstance(m.get("content"), list):
            for c in m["content"]:
                if c.get("type") == "tool_result":
                    results[c["tool_use_id"]] = text_of(c.get("content", ""))
    for t in tools:
        r = results.get(t.pop("id"), "")
        t["result_bytes"] = len(r.encode())
        t["result_head"] = scrub(r, trial)[:600]
    summary = {
        "trial": trial, "batch": int(batch), "models": sorted(x for x in models if x),
        "api_calls": len(calls),
        "usage_sum": {k: sum(c[k] for c in calls) for k in calls[0]} if calls else {},
        "per_call_usage": calls, "tool_calls": len(tools),
        "tool_result_bytes": sum(t["result_bytes"] for t in tools), "tools": tools,
    }
    (out / f"{trial}.json").write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    print(trial, summary["models"], summary["api_calls"], summary["tool_calls"], summary["tool_result_bytes"])
