#!/usr/bin/env python3
"""Check design fixtures and handoff references. This does not qualify a product CLI."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXAMPLE = ROOT / "examples/run-watch"
MOCK_SHA256 = "f483ef2db612b85f085fc403e7912b60b7f21e432535b0e18cc1cdea73eae6c6"

def fixture(name):
    return json.loads((EXAMPLE / name).read_text())

def main():
    spec = (ROOT / "plan/27-run-watch-and-session-handoff.md").read_text()
    start, ack, view = (fixture(name) for name in ("start.json", "start-result.json", "show-34.json"))
    assert start["schema"] == "writfold.run-start/1"
    assert ack["schema"] == "writfold.run-start-result/1"
    assert view["schema"] == "writfold.run-view/1"
    assert ack["request_key"] == start["request_key"] and ack["start_event"] == 1
    assert ack["reused"] is False and ack["run_ref"] == view["run_ref"]
    realm, run = ack["run_ref"].split("/")
    assert realm == start["realm_id"] and all(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]*", v) for v in (realm, run))
    assert ack["watch_argv"] == ["writfold", "run", "watch", ack["run_ref"]]
    assert " ".join(ack["watch_argv"]) in spec
    assert view["corpus_release"] == start["context_release"]
    assert view["procedure_revision"] == start["procedure_revision"]
    assert set(view["repo_vector"]) == set(start["repo_ids"])
    assert view["selected_event"] == view["view_through"] == view["sampled_ledger_head"] == 34
    blocker = view["blocker"]
    assert blocker["matches"] == 2 and view["repo_vector"][blocker["repo"]] == blocker["revision"]
    assert view["run_state"] == "Blocked"
    skill = EXAMPLE / "session-driver/SKILL.md"
    assert "name: writfold-session-driver" in skill.read_text()
    for link in re.findall(r'\[[^]]+\]\(([^)]+)\)', skill.read_text()):
        ref = (skill.parent / link).resolve()
        assert ref.is_relative_to(skill.parent) and ref.is_file()
    assert hashlib.sha256((EXAMPLE / "approved-screens.pdf").read_bytes()).hexdigest() == MOCK_SHA256
    meta = json.loads((ROOT / "registries/bundle.json").read_text())
    assert meta["run_watch_product"] == "not_run" and meta["run_watch_design"] == "accepted-human-baseline"
    print("PASS: synthetic receipt, portable reference, replay snapshot, driver closure and approved screen digest.")
    print("Fixture consistency only. No Writfold binary, provider effect or production proof is exercised.")

if __name__ == "__main__":
    main()
