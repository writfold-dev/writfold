#!/usr/bin/env python3
"""Publishable, scrubbed summary of each trial's transcript: model ids,
per-call usage, every tool call (input and the head of its result), bytes of
tool output. Paths are replaced by placeholders (<trial>, <writfold>,
<scratch>, <home>). Also copies answers, final diffs and recipes/programs.
Usage: summarize.py SCRATCH OUT_DIR"""
import json, pathlib, re, sys

HOME = str(pathlib.Path.home())
S, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])


def scrub(text, trial):
    t = f"{S}/trials/{trial}"
    text = (text.replace(t, "<trial>").replace(f"{S}/bin/writfold", "<writfold>")
            .replace(str(S), "<scratch>").replace(HOME, "<home>"))
    return re.sub(r"/tmp/claude-[0-9]+", "<tmp>", text)


def text_of(c):
    if isinstance(c, str):
        return c
    return "".join(x.get("text", "") for x in c if isinstance(x, dict))


for d in ("raw", "agent-answers", "diffs", "agent-programs", "mutations"):
    (out / d).mkdir(parents=True, exist_ok=True)
for tr in sorted((S / "transcripts").glob("*.jsonl")):
    trial = tr.stem
    models, calls, tools, results, seen, final = set(), [], [], {}, set(), ""
    for line in open(tr, errors="replace"):
        try:
            e = json.loads(line)
        except Exception:
            continue
        m = e.get("message") or {}
        if e.get("type") == "assistant":
            if m.get("model"):
                models.add(m["model"])
            mid = m.get("id")
            if mid not in seen and m.get("usage"):
                seen.add(mid)
                u = m["usage"]
                calls.append({k: u.get(k, 0) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                            "cache_read_input_tokens", "output_tokens")})
            for c in m.get("content", []):
                if isinstance(c, dict) and c.get("type") == "tool_use":
                    inp = c.get("input", {})
                    shown = inp.get("command") or inp.get("file_path") or json.dumps(inp)
                    if c["name"] == "Write":
                        shown = f"{inp.get('file_path')} <= {inp.get('content', '')}"
                    if c["name"] == "Edit":
                        shown = f"{inp.get('file_path')}: {inp.get('old_string', '')!r} -> {inp.get('new_string', '')!r}"
                    if c["name"] == "SubagentHandback":
                        final = inp.get("message", "")
                    tools.append({"id": c["id"], "tool": c["name"], "input": scrub(str(shown), trial)[:3000]})
        elif e.get("type") == "user" and isinstance(m.get("content"), list):
            for c in m["content"]:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    results[c["tool_use_id"]] = (text_of(c.get("content", "")), bool(c.get("is_error")))
    for t in tools:
        r, err = results.pop(t.pop("id"), ("", False))
        t["result_bytes"] = len(r.encode())
        t["is_error"] = err
        t["result_head"] = scrub(r, trial)[:800]
    summary = {"trial": trial, "models": sorted(models), "api_calls": len(calls),
               "usage_sum": {k: sum(c[k] for c in calls) for k in calls[0]} if calls else {},
               "tool_calls": len(tools), "tool_result_bytes": sum(t["result_bytes"] for t in tools),
               "final_reply": scrub(final, trial)[:500], "tools": tools}
    (out / "raw" / f"{trial}.json").write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    d = S / "trials" / trial
    if (d / "answer.txt").exists():
        (out / "agent-answers" / f"{trial}.answer.txt").write_text(scrub((d / "answer.txt").read_text(errors="replace"), trial))
    if (d / "final.diff").exists():
        (out / "diffs" / f"{trial}.diff").write_text(scrub((d / "final.diff").read_text(errors="replace"), trial))
    if (d / "mutation.json").exists():
        (out / "mutations" / f"{trial}.json").write_text(scrub((d / "mutation.json").read_text(), trial))
    for f in sorted((d / "work").glob("*")):
        if f.suffix in (".tuft", ".spl") and f.is_file():
            (out / "agent-programs" / f"{trial}.{f.name}").write_text(scrub(f.read_text(errors="replace"), trial))
    print(trial, sorted(models), len(calls), len(tools))
