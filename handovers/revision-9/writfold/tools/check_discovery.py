#!/usr/bin/env python3
"""Reproduce the finite discovery checks and verify retained source identities."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
Q = ROOT / 'qualification/similar-cases'
E = ROOT / 'evidence/similar-cases'

def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    record=json.loads((E/'receipts.json').read_text())
    assert record['product_qualification']=='not_run'
    assert record['lean']['sha256']==digest(Q/'formal/CaseDiscovery.lean')
    assert record['tla']['sha256']==digest(Q/'formal/IndexPublication.tla')
    for checked in record['metamath_checker']['proofs']:
        path=ROOT/checked['source']
        assert checked['sha256']==digest(path)
        assert checked['positive_exit']==0 and checked['negative_exit']!=0
        assert ('TraceErasure' not in path.name or 'trace-erasure' in checked['verified_fields'])
    bounded=subprocess.check_output([sys.executable,'-B',str(Q/'check_protocol.py')],text=True)
    assert json.loads(bounded)==record['reference']==json.loads((E/'bounded-reference.json').read_text())
    examples=subprocess.check_output([sys.executable,'-B',str(Q/'check_reader.py')],text=True)
    assert examples==(E/'reader.log').read_text()
    print('PASS: proof source hashes, bounded reference, mutation counterexamples and reader fixtures.')
    print('Lean/TLC/TLAPS, native qualification and token evaluation remain open.')

if __name__=='__main__':
    main()
