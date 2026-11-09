#!/usr/bin/env python3
"""Build results.json from grades, harness usage reports and transcript summaries.
Usage: analyze.py GRADES_JSON USAGE_TXT RAW_DIR ORDER_JSON -> results.json on stdout"""
import json, pathlib, statistics as st, sys
grades = {g["trial"]: g for g in json.load(open(sys.argv[1]))}
usage = {}
for line in open(sys.argv[2]):
    t, tok, tools, ms = line.split()
    usage[t] = {"subagent_tokens": int(tok), "tool_uses": int(tools), "duration_ms": int(ms)}
raw = {p.stem: json.loads(p.read_text()) for p in pathlib.Path(sys.argv[3]).glob("*.json")}
order = json.load(open(sys.argv[4]))["order"]
deviations = {"P2-B-r2": "piped one Tuft run's output through `grep write_objects` (filtering tool output, not searching the repository); kept in both denominators"}
trials = []
for t in order:
    g, u, r = grades[t], usage[t], raw[t]
    trials.append({
        "trial": t, "task": g["task"], "arm": g["arm"], "repeat": g["repeat"],
        "launch_position": order.index(t) + 1, "batch": r["batch"],
        "success": g["success"], "found": g["found"], "expected": g["expected"],
        "missing": g["missing"], "extra": g["extra"], "wrong_text": g["wrong_text"],
        **u,
        "tool_calls_excluding_handback": sum(1 for x in r["tools"] if x["tool"] != "SubagentHandback"),
        "tool_output_bytes": r["tool_result_bytes"],
        "api_calls": r["api_calls"], "output_tokens": r["usage_sum"]["output_tokens"],
        "model": r["models"], "deviation": deviations.get(t),
    })
def arm(a):
    xs = [x for x in trials if x["arm"] == a]
    s = lambda k: sum(x[k] for x in xs)
    return {"trials": len(xs), "successes": sum(x["success"] for x in xs),
            "subagent_tokens": s("subagent_tokens"), "mean_subagent_tokens": round(s("subagent_tokens") / len(xs), 1),
            "tool_uses": s("tool_uses"), "tool_calls_excluding_handback": s("tool_calls_excluding_handback"),
            "tool_output_bytes": s("tool_output_bytes"), "output_tokens": s("output_tokens"),
            "duration_ms": s("duration_ms"), "arm_b_read_language_reference": None}
summary = {"A": arm("A"), "B": arm("B")}
summary["B"]["arm_b_read_language_reference"] = 0
del summary["A"]["arm_b_read_language_reference"]
paired = []
for task in sorted({x["task"] for x in trials}):
    cell = lambda a, k: st.mean(x[k] for x in trials if x["task"] == task and x["arm"] == a)
    row = {"task": task}
    for k in ("subagent_tokens", "tool_uses", "tool_output_bytes", "output_tokens", "duration_ms"):
        row[f"{k}_A"], row[f"{k}_B"] = cell("A", k), cell("B", k)
        row[f"{k}_B_minus_A"] = cell("B", k) - cell("A", k)
    row["successes_A"] = sum(x["success"] for x in trials if x["task"] == task and x["arm"] == "A")
    row["successes_B"] = sum(x["success"] for x in trials if x["task"] == task and x["arm"] == "B")
    paired.append(row)
def dist(k):
    d = [p[f"{k}_B_minus_A"] for p in paired]
    return {"mean": round(st.mean(d), 1), "min": min(d), "max": max(d), "tasks_B_lower": sum(x < 0 for x in d)}
summary["paired_B_minus_A_per_task"] = {k: dist(k) for k in ("subagent_tokens", "tool_uses", "tool_output_bytes", "output_tokens", "duration_ms")}
summary["B_vs_A_token_ratio"] = round(summary["B"]["subagent_tokens"] / summary["A"]["subagent_tokens"], 4)
summary["token_win_claimed"] = summary["B"]["subagent_tokens"] < summary["A"]["subagent_tokens"] and summary["B"]["successes"] >= summary["A"]["successes"]
print(json.dumps({
 "schema": "writfold-projection-pilot/1", "date": "2026-11-09", "kind": "debug pilot (not a headline result)",
 "question": "Do Tuft projections let an agent answer evidence-finding questions with fewer tokens than grep-and-read, at equal or better correctness?",
 "corpus": {"repository": "writfold (this repository)", "commit": "9e0c8d648852e2e6f27c6f9517c71d8e5ea67a49", "tracked_files": 1664},
 "model": "Claude Sonnet via Claude Code general-purpose subagents (alias 'sonnet'; transcripts report model id claude-sonnet-5-5)",
 "usage_source": "subagent_tokens, tool_uses and duration_ms: Claude Code's per-subagent usage report (total per trial, includes fixed host context; tool_uses includes the final hand-back call). tool_output_bytes, api_calls and output_tokens: computed from each subagent transcript",
 "arms": {"A": "ordinary tools: Bash (grep, rg, git grep, sed) and Read",
          "B": "writfold tuft run for searching; Read/sed only when Tuft output is not enough; no grep/rg/git grep/find. Tool note carries a short Tuft quick reference; trial init done by the harness outside the measured cost"},
 "writfold_binary": {"built_from": "9e0c8d648852e2e6f27c6f9517c71d8e5ea67a49", "sha256": "eff75cc2beb570dfefe28f22a4af0d188e4395206d70fd3fa486cd60f0e7d03f"},
 "repeats": 2, "randomization": {"seed": 20261109, "order": order, "concurrency": 3},
 "summary": summary, "paired_per_task": paired, "trials": trials}, indent=1))
