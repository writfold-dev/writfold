#!/usr/bin/env python3
"""Check illustrative canonical fixtures and structural engineering registries.

Passing this program validates this planning aid only. It does not verify the
new calculus, production Rust, a decoder, effect adapters or product gates.
"""
from pathlib import Path
import json
import sys
sys.dont_write_bytecode = True
from engineering_codec import canonical, fingerprint

ROOT = Path(__file__).resolve().parents[1]
vectors = [
    (None, 'f6'), (False, 'f4'), (True, 'f5'), (0, '00'), (23, '17'),
    (24, '1818'), (256, '190100'), (-1, '20'), (-25, '3818'),
    (2**64 - 1, '1bffffffffffffffff'), (-2**64, '3bffffffffffffffff'),
    (2**64, 'c249010000000000000000'),
    (-2**64 - 1, 'c349010000000000000000'),
    (b'ab', '426162'), ('a', '6161'), ('é', '62c3a9'),
    ([1, 'a'], '82016161'), ({'aa': 2, 'b': 1}, 'a261620162616102')]
for value, expected in vectors:
    assert canonical(value).hex() == expected, repr(value)
for value in (1.2, float('nan'), {'x': (1, 2)}, {1: 'x'}, '\ud800'):
    try:
        canonical(value)
    except (ValueError, UnicodeError):
        pass
    else:
        raise AssertionError('accepted unsupported value')
payload = {'schema_version': 1, 'name': 'demo', 'resources': ['entry.md']}
assert fingerprint('skill', payload) != fingerprint('procedure', payload)
assert fingerprint('skill', payload) == fingerprint('skill', dict(reversed(list(payload.items()))))
assert fingerprint('skill', payload) != fingerprint('skill', {**payload, 'resources': ['entry.md', 'rust.md']})
fixtures = json.loads((ROOT/'examples/engineering/fingerprints.json').read_text())
for row in fixtures:
    assert canonical(row['payload']).hex() == row['canonical_hex'], row['name']
    assert fingerprint(row['kind'], row['payload']) == row['fingerprint'], row['name']
theorems = json.loads((ROOT/'registries/engineering-theorems.json').read_text())
assert {r['id'] for r in theorems} == {f'E{i:02}' for i in range(1, 13)}
assert all(r['status'] == 'not_run' and not r['theorem_symbols'] and r['receipt'] is None for r in theorems)
print(f'PASS: {len(vectors)} hand-encoded vectors; 5 unsupported inputs; domain/order/change checks; {len(fixtures)} golden fingerprints; 12 unexecuted theorem targets.')
print('Planning fixture checks only. No production/parser/Lean/Verus/TLA+/TLAPS gate is passed.')
