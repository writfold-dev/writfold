#!/usr/bin/env python3
"""Build results.json for the main stage from its grades, usage and order,
and apply the pre-registered decision rule (README, "Decision rule"; the
reliability pilot's rule verbatim). Adapted from the pilot's analyze.py:
tasks come from order.json's main stage; adds per-family results, failure
classes, the precondition harm sub-metric and Splicery flow measures.

Usage: analyze.py GRADES_JSON USAGE_TXT ORDER_JSON > results.json
USAGE_TXT rows: <trial> <subagent_tokens> <tool_uses> <duration_ms>."""
import json, math, random, statistics as st, sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from tasks import FAMILIES, TASKS  # noqa: E402

grades = {g["trial"]: g for g in json.load(open(sys.argv[1]))}
usage = {}
for line in open(sys.argv[2]):
    if line.strip() and not line.startswith("#"):
        t, tok, tools, ms = line.split()[:4]
        usage[t] = {"subagent_tokens": int(tok), "tool_uses": int(tools), "duration_ms": int(ms)}
main = json.load(open(sys.argv[3]))["main"]
order, TASK_IDS = main["order"], main["tasks"]
trials = []
for t in order:
    g = grades.get(t)
    if g is None:  # never graded: counts as a failure in both denominators
        _, task, a, r = t.split("-")
        g = {"trial": t, "task": task, "family": TASKS[task]["family"], "arm": a, "repeat": int(r[1:]),
             "clean": False, "correct": False, "missing_grade": True, "failure_classes": ["not graded"]}
    trials.append({**g, **usage.get(t, {"subagent_tokens": None, "tool_uses": None, "duration_ms": None})})
models = sorted({m for x in trials for m in x.get("models", [])})
HARM = ["realized_out_of_scope", "realized_clobber", "realized_evidence_less", "realized_precondition_violated",
        "attempted_out_of_scope", "attempted_clobber", "attempted_evidence_less"]
CLASSES = ["wrong answer", "unsupported claim", "realized harm", "operational trip", "repeated errors"]


def count(xs, k):
    return sum(1 for x in xs if x.get(k) or x.get(k + "_calls"))


def arm(a, xs=None):
    xs = [x for x in (xs if xs is not None else trials) if x["arm"] == a]
    tsum = lambda k: sum(x.get(k) or 0 for x in xs)
    find = [x for x in xs if x.get("recall") is not None]
    bt = [x for x in xs if x.get("tuft_runs")]
    bp = [x for x in xs if x.get("prepare_runs")]
    out = {"trials": len(xs), "clean": [sum(bool(x.get("clean")) for x in xs), len(xs)],
           "correct": [sum(bool(x.get("correct")) for x in xs), len(xs)],
           "trials_with_unsupported_claims": sum(1 for x in xs if (x.get("unsupported_claims") or 0) > 0),
           "unsupported_claims": tsum("unsupported_claims"),
           "realized_harm": sum(bool(x.get("realized_harm")) for x in xs),
           "attempted_harm": sum(bool(x.get("attempted_harm")) for x in xs),
           **{k: count(xs, k) for k in HARM},
           "refused_write_trials": sum(1 for x in xs if x.get("refused_write_calls")),
           "deviation_trials": sum(1 for x in xs if x.get("deviation")),
           "silent_partial": sum(1 for x in xs if x.get("silent_partial")),
           "failure_classes": {c: sum(1 for x in xs if c in (x.get("failure_classes") or [])) for c in CLASSES},
           "find_recall_mean": round(st.mean(x["recall"] for x in find), 4) if find else None,
           "failing_calls_harness": sum((x.get("failing_calls") or {}).get("harness", 0) for x in xs),
           "failing_calls_agent": sum((x.get("failing_calls") or {}).get("agent", 0) for x in xs),
           "repeated_failing_agent_calls": sum((x.get("failing_calls") or {}).get("repeated_agent", 0) for x in xs),
           "tuft_trials": len(bt), "tuft_first_run_success": sum(1 for x in bt if x.get("tuft_first_run_success")),
           "tuft_first_run_syntax_trip": sum(1 for x in bt if x.get("tuft_first_run_syntax_trip")),
           "tuft_runs": tsum("tuft_runs"), "tuft_syntax_errors": tsum("tuft_syntax_errors"),
           "prepare_trials": len(bp), "prepare_first_run_success": sum(1 for x in bp if x.get("prepare_first_run_success")),
           "prepare_runs": tsum("prepare_runs"), "prepare_failed_runs": tsum("prepare_failed_runs"),
           "recipe_errors": tsum("recipe_errors"), "approve_apply_failed_runs": tsum("approve_apply_failed_runs"),
           "syntax_trip_trials": sum(1 for x in xs if x.get("syntax_trip")),
           "subagent_tokens": tsum("subagent_tokens"),
           "tokens_complete": all(x.get("subagent_tokens") is not None for x in xs),
           "tool_calls_excluding_handback": tsum("tool_calls_excluding_handback"),
           "tool_output_bytes": tsum("tool_output_bytes"), "duration_ms": tsum("duration_ms")}
    return out


summary = {"A": arm("A"), "B": arm("B")}
per_task = []
for t in TASK_IDS:
    row = {"task": t, "family": TASKS[t]["family"], "kind": TASKS[t]["kind"]}
    for a in "AB":
        xs = sorted((x for x in trials if x["task"] == t and x["arm"] == a), key=lambda x: x["repeat"])
        row[f"clean_{a}"] = sum(bool(x.get("clean")) for x in xs)
        row[f"pass3_{a}"] = len(xs) == 3 and all(x.get("clean") for x in xs)
        row[f"first_attempt_clean_{a}"] = bool(xs and xs[0].get("clean"))
        row[f"realized_harm_{a}"] = sum(bool(x.get("realized_harm")) for x in xs)
        row[f"unsupported_{a}"] = sum(x.get("unsupported_claims") or 0 for x in xs)
        row[f"mean_tokens_{a}"] = round(st.mean(x["subagent_tokens"] for x in xs), 1) \
            if xs and all(x.get("subagent_tokens") for x in xs) else None
    per_task.append(row)
per_family = {}
for f in sorted({TASKS[t]["family"] for t in TASK_IDS}):
    xs = [x for x in trials if x["family"] == f]
    per_family[f] = {"name": FAMILIES[f], "tasks": [t for t in TASK_IDS if TASKS[t]["family"] == f],
                     **{a: {k: arm(a, xs)[k] for k in ("clean", "realized_harm", "trials_with_unsupported_claims", "failure_classes")}
                        for a in "AB"}}

nA, nB = summary["A"]["clean"][1], summary["B"]["clean"][1]
cA, cB = summary["A"]["clean"][0], summary["B"]["clean"][0]
diff = cB / nB - cA / nA


def boot(seed=20261112, reps=10000):
    rng = random.Random(seed)
    cells = {t: ([bool(x.get("clean")) for x in trials if x["task"] == t and x["arm"] == "A"],
                 [bool(x.get("clean")) for x in trials if x["task"] == t and x["arm"] == "B"]) for t in TASK_IDS}
    ds = []
    for _ in range(reps):
        pick = [rng.choice(TASK_IDS) for _ in TASK_IDS]
        a = [v for t in pick for v in cells[t][0]]
        b = [v for t in pick for v in cells[t][1]]
        ds.append(sum(b) / len(b) - sum(a) / len(a))
    ds.sort()
    return ds[int(0.025 * reps)], ds[int(0.975 * reps) - 1]


def fisher(a, n1, b, n2):
    k, N = a + b, n1 + n2
    p = lambda x: math.comb(n1, x) * math.comb(n2, k - x) / math.comb(N, k)
    obs = p(a)
    return sum(p(x) for x in range(max(0, k - n2), min(k, n1) + 1) if p(x) <= obs * (1 + 1e-9))


lo, hi = boot()
harm_worse_B = [k for k in HARM if summary["B"][k] > summary["A"][k]]
harm_worse_A = [k for k in HARM if summary["A"][k] > summary["B"][k]]
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
print(json.dumps({
    "schema": "writfold-reliability-r2/1", "date": "2026-11-11", "stage": "main (haiku)",
    "corpus": {"repository": "https://github.com/encode/httpx.git", "commit": "b5addb64f0161ff6bfe94c124ef76f6a1fba5254"},
    "model_alias": "haiku", "resolved_models": models, "models_consistent": len(models) == 1,
    "summary": summary,
    "primary": {"clean_A": [cA, nA], "clean_B": [cB, nB], "B_minus_A_pp": round(100 * diff, 1),
                "task_clustered_bootstrap_95ci_pp": [round(100 * lo, 1), round(100 * hi, 1)],
                "bootstrap": {"seed": 20261112, "reps": 10000,
                              "resample": f"{len(TASK_IDS)} tasks with replacement, each with its 3 A and 3 B trials"},
                "fisher_two_sided_p": round(fisher(cB, nB, cA, nA), 4),
                "harm_submetrics_worse_in_B": harm_worse_B, "harm_submetrics_worse_in_A": harm_worse_A,
                "deviation_trials_A": devA, "deviation_trials_B": devB},
    "verdict": verdict,
    "staging": {"arm_A_at_ceiling": too_easy, "arm_B_unclean": len(b_unclean),
                "arm_B_unclean_with_syntax_trip": sum(bool(x.get("syntax_trip")) for x in b_unclean),
                "syntax_dominated": syntax_dominated},
    "per_family": per_family, "per_task": per_task, "trials": trials}, indent=1, ensure_ascii=False))
