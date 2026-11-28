#!/usr/bin/env python3
"""Tests for proofs/law_coverage.py (#206): each refusal on a small
fixture project. Run by the quality gate: python3 proofs/test_law_coverage.py
"""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import law_coverage  # noqa: E402

FILES = {
    "Extracted/WritfoldCore/Types.lean":
        "inductive k.Word where\n| A : k.Word\n\ninductive k.Other where\n| B : k.Other\n",
    "Extracted/WritfoldCore/K.lean":
        "def k.decide (x : Bool) : Result k.Word := do\n  ok k.Word.A\n\n"
        "def k.helper (x : Bool) : Result Bool := do\n  ok x\n\n"
        "def k.decide_loop (x : Bool) : Result Bool := do\n  ok x\n",
    "Writfold/Conform/Words.lean":
        "namespace Writfold.Core.Conform\n\ndef wordOf : k.Word → Calculus.Tri\n  | .A => .yes\n\n"
        "theorem wordOf_onto (t : Calculus.Tri) : True := trivial\n\nend Writfold.Core.Conform\n",
    "Writfold/Conform/Links.lean":
        "namespace Writfold.Core.Conform\n\n"
        "theorem decide_implements (x : Bool) :\n    k.decide x ⦃ w => wordOf w = Calculus.admit x ⦄ := by\n  sorry\n\n"
        "end Writfold.Core.Conform\n",
    "Calculus/Word.lean":
        "namespace Calculus\n\ntheorem admit_law (x : Bool) : admit x = admit x := rfl\n\nend Calculus\n",
    "Audit.lean":
        "def claims : List Name := [\n  ``Writfold.Core.Conform.decide_implements,\n]\n",
    "laws.toml":
        '[[word]]\ntype = "k.Word"\ncore = "Calculus.Tri"\nabstraction = "wordOf"\nonto = "wordOf_onto"\n\n'
        '[[local]]\ntype = "k.Other"\nwhy = "a fixture type"\n\n'
        '[[link]]\nfunction = "k.decide"\nimplements = "Calculus.admit"\n'
        'theorem = "Writfold.Core.Conform.decide_implements"\nlaws = ["Calculus.admit_law"]\n\n'
        '[[out]]\nfunction = "k.helper"\nwhy = "a fixture helper"\n',
}


class LawCoverage(unittest.TestCase):
    def project(self, **changes: str | None) -> Path:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        for name, text in {**FILES, **changes}.items():
            if text is None:
                continue
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        return root

    def refused(self, needle: str, **changes: str | None) -> None:
        errors, _ = law_coverage.check(self.project(**changes))
        self.assertTrue(any(needle in e for e in errors), f"{needle!r} not in {errors}")

    def test_the_fixture_passes(self) -> None:
        errors, line = law_coverage.check(self.project())
        self.assertEqual(errors, [])
        self.assertEqual(line, "LAW_COVERAGE judged=1 declared_out=1 remainder=0 of 2")

    def test_a_new_function_with_no_entry(self) -> None:
        extra = FILES["Extracted/WritfoldCore/K.lean"] + "\ndef k.fresh (x : Bool) : Result Bool := do\n  ok x\n"
        self.refused("k.fresh: translated, but neither linked", **{"Extracted/WritfoldCore/K.lean": extra})

    def test_nothing_is_left_unaccounted(self) -> None:
        laws = FILES["laws.toml"].replace('[[out]]\nfunction = "k.helper"\nwhy = "a fixture helper"\n', "")
        self.refused("k.helper: translated, but neither linked nor declared out", **{"laws.toml": laws})
        self.refused("the backlog is closed", **{"unlinked.txt": "# entries: 1\nk.helper\n"})
        gone = FILES["laws.toml"] + '\n[[out]]\nfunction = "k.gone"\nwhy = "removed"\n'
        self.refused("not a translated function", **{"laws.toml": gone})

    def test_an_out_needs_a_reason(self) -> None:
        laws = FILES["laws.toml"].replace('why = "a fixture helper"', 'why = " "')
        self.refused("declared out without a reason", **{"laws.toml": laws})

    def test_a_word_result_may_not_be_out(self) -> None:
        laws = FILES["laws.toml"] + '\n[[out]]\nfunction = "k.decide"\nwhy = "no"\n'
        self.refused("may not be declared out", **{"laws.toml": laws})

    def test_a_new_enum_needs_an_entry(self) -> None:
        types = FILES["Extracted/WritfoldCore/Types.lean"] + "\ninductive k.New where\n| C : k.New\n"
        self.refused("type k.New: neither", **{"Extracted/WritfoldCore/Types.lean": types})

    def test_a_link_must_name_its_operation_and_function(self) -> None:
        links = FILES["Writfold/Conform/Links.lean"].replace("Calculus.admit x", "true")
        self.refused("does not name Calculus.admit", **{"Writfold/Conform/Links.lean": links})
        links = FILES["Writfold/Conform/Links.lean"].replace("k.decide x", "k.helper x")
        self.refused("is not stated over k.decide", **{"Writfold/Conform/Links.lean": links})

    def test_a_link_theorem_is_a_required_claim(self) -> None:
        self.refused("is not a required claim", **{"Audit.lean": "def claims : List Name := [\n]\n"})

    def test_a_law_must_be_a_calculus_theorem_about_the_operation(self) -> None:
        self.refused("is not a theorem under proofs/Calculus",
                     **{"Calculus/Word.lean": "namespace Calculus\nend Calculus\n"})
        self.refused("does not mention Calculus.admit",
                     **{"Calculus/Word.lean": "namespace Calculus\n\ntheorem admit_law : True := trivial\n\nend Calculus\n"})

    def test_an_abstraction_needs_its_onto_theorem(self) -> None:
        words = FILES["Writfold/Conform/Words.lean"].replace("theorem wordOf_onto", "theorem other")
        self.refused("no onto theorem", **{"Writfold/Conform/Words.lean": words})


if __name__ == "__main__":
    unittest.main()
