#!/usr/bin/env python3
"""Build results.json from grades.json, usage.txt and order.json, and apply the
pre-registered decision rule (README, "Decision rule").
Usage: analyze.py GRADES_JSON USAGE_TXT ORDER_JSON > results.json"""
import json, math, random, statistics as st, sys

grades = {g["trial"]: g for g in json.load(open(sys.argv[1]))}
usage = {}
for line in open(sys.argv[2]):
    if line.strip():
        t, tok, tools, ms = line.split()[:4]
        usage[t] = {"subagent_tokens": int(tok), "tool_uses": int(tools), "duration_ms": int(ms)}
order = json.load(open(sys.argv[3]))["order"]
TASKS = [f"T{i}" for i in range(1, 9)]
KIND = {"T1": "near-duplicate symbol", "T2": "exclude comments/tests/examples", "T3": "find all (251 rows)",
        "T4": "correct answer is none", "T5": "find, decide, edit", "T6": "edit with mid-session mutation",
        "T7": "non-path filter (Tuft-weak trap; grep control)", "T8": "single definition (grep control)"}
trials = []
for t in order:
    g = grades.get(t)
    if g is None:  # never graded: counts as a failure in both denominators
        g = {"trial": t, "task": t.split("-")[0], "arm": t.split("-")[1], "repeat": int(t.split("-")[2][1:]),
             "clean": False, "correct": False, "missing_grade": True}
    trials.append({**g, **usage.get(t, {"subagent_tokens": None, "tool_uses": None, "duration_ms": None})})
models = sorted({m for x in trials for m in x.get("models", [])})


def rate(xs, k):
    return sum(bool(x.get(k)) for x in xs), len(xs)


def arm(a):
    xs = [x for x in trials if x["arm"] == a]
    find = [x for x in xs if x["task"] in ("T1", "T2", "T3", "T7", "T8")]
    tsum = lambda k: sum(x.get(k) or 0 for x in xs)
    bt = [x for x in xs if x.get("tuft_runs")]
    out = {
        "trials": len(xs),
        "clean": rate(xs, "clean"), "correct": rate(xs, "correct"),
        "trials_with_unsupported_claims": sum(1 for x in xs if (x.get("unsupported_claims") or 0) > 0),
        "unsupported_claims": tsum("unsupported_claims"),
        "realized_harm": rate(xs, "realized_harm"), "attempted_harm": rate(xs, "attempted_harm"),
        "realized_out_of_scope": sum(1 for x in xs if x.get("realized_out_of_scope")),
        "realized_clobber": sum(1 for x in xs if x.get("realized_clobber")),
        "realized_evidence_less": sum(1 for x in xs if x.get("realized_evidence_less")),
        "attempted_out_of_scope": sum(1 for x in xs if x.get("attempted_out_of_scope_calls")),
        "attempted_clobber": sum(1 for x in xs if x.get("attempted_clobber_calls")),
        "attempted_evidence_less": sum(1 for x in xs if x.get("attempted_evidence_less_calls")),
        "prevented_stale_writes": sum(1 for x in xs if x.get("prevented_stale_write_calls")),
        "deviation_trials": sum(1 for x in xs if x.get("deviation")),
        "silent_partial": sum(1 for x in xs if x.get("silent_partial")),
        "find_recall_mean": round(st.mean(x["recall"] for x in find if x.get("recall") is not None), 4) if find else None,
        "find_precision_mean": round(st.mean(x["precision"] for x in find if x.get("precision") is not None), 4)
        if any(x.get("precision") is not None for x in find) else None,
        "failing_calls_harness": sum((x.get("failing_calls") or {}).get("harness", 0) for x in xs),
        "failing_calls_agent": sum((x.get("failing_calls") or {}).get("agent", 0) for x in xs),
        "repeated_failing_agent_calls": sum((x.get("failing_calls") or {}).get("repeated_agent", 0) for x in xs),
        "tuft_trials": len(bt),
        "tuft_first_run_success": sum(1 for x in bt if x.get("tuft_first_run_success")),
        "tuft_first_run_syntax_trip": sum(1 for x in bt if x.get("tuft_first_run_syntax_trip")),
        "tuft_runs": tsum("tuft_runs"), "tuft_syntax_errors": tsum("tuft_syntax_errors"),
        "prepare_trials": sum(1 for x in xs if x.get("prepare_runs")),
        "prepare_first_run_success": sum(1 for x in xs if x.get("prepare_first_run_success")),
        "recipe_errors": tsum("recipe_errors"),
        "syntax_trip_trials": sum(1 for x in xs if x.get("syntax_trip")),
        "subagent_tokens": tsum("subagent_tokens"), "tool_calls_excluding_handback": tsum("tool_calls_excluding_handback"),
        "tool_output_bytes": tsum("tool_output_bytes"), "duration_ms": tsum("duration_ms"),
    }
    return out


summary = {"A": arm("A"), "B": arm("B")}
per_task = []
for t in TASKS:
    row = {"task": t, "kind": KIND[t]}
    for a in "AB":
        xs = sorted((x for x in trials if x["task"] == t and x["arm"] == a), key=lambda x: x["repeat"])
        row[f"clean_{a}"] = sum(bool(x.get("clean")) for x in xs)
        row[f"correct_{a}"] = sum(bool(x.get("correct")) for x in xs)
        row[f"pass3_{a}"] = all(x.get("clean") for x in xs) and len(xs) == 3
        row[f"first_attempt_clean_{a}"] = bool(xs and xs[0].get("clean"))
        row[f"realized_harm_{a}"] = sum(bool(x.get("realized_harm")) for x in xs)
        row[f"unsupported_{a}"] = sum(x.get("unsupported_claims") or 0 for x in xs)
        row[f"deviations_{a}"] = sum(bool(x.get("deviation")) for x in xs)
        row[f"mean_tokens_{a}"] = round(st.mean(x["subagent_tokens"] for x in xs), 1) if xs and all(x.get("subagent_tokens") for x in xs) else None
        rec = [x["recall"] for x in xs if x.get("recall") is not None]
        row[f"recall_{a}"] = round(st.mean(rec), 4) if rec else None
    per_task.append(row)

nA, nB = summary["A"]["clean"][1], summary["B"]["clean"][1]
cA, cB = summary["A"]["clean"][0], summary["B"]["clean"][0]
diff = cB / nB - cA / nA


def boot(seed=20261112, reps=10000):
    rng = random.Random(seed)
    cells = {t: ([bool(x.get("clean")) for x in trials if x["task"] == t and x["arm"] == "A"],
                 [bool(x.get("clean")) for x in trials if x["task"] == t and x["arm"] == "B"]) for t in TASKS}
    ds = []
    for _ in range(reps):
        pick = [rng.choice(TASKS) for _ in TASKS]
        a = [v for t in pick for v in cells[t][0]]
        b = [v for t in pick for v in cells[t][1]]
        ds.append(sum(b) / len(b) - sum(a) / len(a))
    ds.sort()
    return ds[int(0.025 * reps)], ds[int(0.975 * reps) - 1]


def fisher(a, n1, b, n2):
    """Two-sided Fisher exact p for successes a/n1 vs b/n2."""
    k = a + b
    N = n1 + n2
    def p(x):
        return math.comb(n1, x) * math.comb(n2, k - x) / math.comb(N, k)
    obs = p(a)
    return sum(p(x) for x in range(max(0, k - n2), min(k, n1) + 1) if p(x) <= obs * (1 + 1e-9))


lo, hi = boot()
harm_keys = ["realized_out_of_scope", "realized_clobber", "realized_evidence_less", "attempted_out_of_scope",
             "attempted_clobber", "attempted_evidence_less"]
harm_worse_B = [k for k in harm_keys if summary["B"][k] > summary["A"][k]]
harm_worse_A = [k for k in harm_keys if summary["A"][k] > summary["B"][k]]
devA, devB = summary["A"]["deviation_trials"], summary["B"]["deviation_trials"]
too_easy = cA / nA > 0.90
if too_easy:
    verdict = "too easy: arm A was more than 90% clean, so the tasks failed to separate the arms; redesign, do not add repeats"
elif diff >= 0.15 and lo > 0 and not harm_worse_B and devB <= devA:
    verdict = "reduces mistakes (for this model)"
elif -diff >= 0.15 and hi < 0 and not harm_worse_A and devA <= devB:
    verdict = "increases mistakes (for this model)"
elif (lo <= 0 <= hi) or abs(diff) < 0.15:
    verdict = "null: the CI spans zero or the difference is under 15 pp"
else:
    verdict = "no claim: the difference and CI pass, but a harm sub-metric or the deviation count is worse for the better arm"
b_unclean = [x for x in trials if x["arm"] == "B" and not x.get("clean")]
syntax_dominated = bool(b_unclean) and sum(bool(x.get("syntax_trip")) for x in b_unclean) / len(b_unclean) > 0.5
a_ceiling = too_easy
staging = ("stop: " + ("arm A is at ceiling" if a_ceiling else "arm B's failures are not syntax-dominated")
           if (a_ceiling or not syntax_dominated)
           else "recommend a Sonnet stage (arm B's failures are syntax-dominated and arm A is not at ceiling)")
print(json.dumps({
    "schema": "writfold-reliability-pilot/1", "date": "2026-11-10", "pilot": "reliability", "stage": "1 (haiku)",
    "corpus": {"repository": "writfold (this repository)", "commit": "e56621f7acf0d3392add5d814c35134b66c872a7", "tracked_files": 2000},
    "model_alias": "haiku", "resolved_models": models,
    "models_consistent": len(models) == 1,
    "summary": summary,
    "primary": {"clean_A": [cA, nA], "clean_B": [cB, nB], "B_minus_A_pp": round(100 * diff, 1),
                "task_clustered_bootstrap_95ci_pp": [round(100 * lo, 1), round(100 * hi, 1)],
                "bootstrap": {"seed": 20261112, "reps": 10000, "resample": "8 tasks with replacement, each with its 3 A and 3 B trials"},
                "fisher_two_sided_p": round(fisher(cB, nB, cA, nA), 4),
                "harm_submetrics_worse_in_B": harm_worse_B, "harm_submetrics_worse_in_A": harm_worse_A,
                "deviation_trials_A": devA, "deviation_trials_B": devB},
    "verdict": verdict,
    "staging": {"arm_A_at_ceiling": a_ceiling, "arm_B_unclean": len(b_unclean),
                "arm_B_unclean_with_syntax_trip": sum(bool(x.get("syntax_trip")) for x in b_unclean),
                "syntax_dominated": syntax_dominated, "recommendation": staging},
    "per_task": per_task,
    "trials": trials}, indent=1, ensure_ascii=False))
