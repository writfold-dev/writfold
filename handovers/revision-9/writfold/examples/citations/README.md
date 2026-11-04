# First regression

Run `python3 tools/reproduce_duplicate_citation.py` from the extracted handover root using Python 3.12+ and Git on Linux/WSL. The probe validates the archived kctx source identity, creates an isolated temporary repository and runs the legacy citation constructor/classifier over the fixture. It asserts the original historical content, both normalized matches, and the advisory selection of line 2. Generated commits and files live only in the temporary directory.

The fixture separates the observed legacy result from the required revision-6 outcome. The production reader, Lean theorems and Verus implementation remain pending. Missing Git, unsupported Python, or a mismatched archive is an explicit probe failure.

Use `duplicate-line.json` for the first CIT03/CIT04 negative case. Add Unicode offsets, deleted/missing source, incomplete scans, duplicate/copy, stale alias, independent witness and policy-change cases in the implementing repository. Count actual model tokens on repeated tasks under the frozen benchmark in plan/26.
