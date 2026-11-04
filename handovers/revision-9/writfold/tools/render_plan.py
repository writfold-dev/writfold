#!/usr/bin/env python3
"""Regenerate stage Markdown and editable Mermaid from the owned stage registry."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[1]
for stage in json.loads((ROOT / 'registries/stages.json').read_text()):
    sid = stage['id']
    tasks = '\n'.join(f'{i}. {s}' for i, s in enumerate(stage['tasks'], 1))
    deliver = '\n'.join('- ' + s for s in stage['deliverables'])
    accept = '\n'.join('- ' + s for s in stage['acceptance'])
    text = f'''# {sid} — {stage['title']}

{stage['goal']}

Status: **not implemented by this plan**. Dependencies: {', '.join(stage['dependencies']) or 'none'}.

![{stage['title']}](../diagrams/{sid}.png)

## Work in order

{tasks}

## Required deliverables

{deliver}

## Exit gate

{accept}

Mandatory gates: {', '.join(stage['gates'])}. Required proof/model entries: {', '.join(stage['proofs']) or 'none at this stage'}. Additional scenarios: {', '.join(stage['scenarios'])}. All applicable existing G/Q/D/R behaviors remain required.

## Evidence and recovery

Use plan/08 for gate receipts, plan/11 for workflow-3 authority/version rules and plan/12 for playbook adoption. Plans/13–16 own views, reading design, the Omnigraph-first decision and storage-epoch contract. Plans/17-23 add engineering layout, fingerprints, skills, execution, learning and the proposed calculus. Plans/24-26 and PRIORITIES.md govern all new proposal admissions. Preserve exact source/tool/fixture hashes, failed attempts and successful replacements. A failed gate does not authorize rewriting current user state or weakening a contract. Keep the stage diagram and atlas explanation current under WF21.

Finish with a cohesive commit, updated traceability and a concise report of delivered behavior, evidence and remaining blockers. Planned command names or created files do not pass a product stage.
'''
    (ROOT / stage['file']).write_text(text)
    lines = ['flowchart TD']
    for i, node in enumerate(stage['nodes']):
        label = node.replace('|', ' — ').replace('"', "'")
        lines.append(f'  N{i}["{label}"]')
    for a, b, label in stage['edges']:
        edge = f'-->|"{label}"|' if label else '-->'
        lines.append(f'  N{a} {edge} N{b}')
    (ROOT / 'diagrams' / f'{sid}.mmd').write_text('\n'.join(lines) + '\n')
print('Stage instructions and Mermaid diagrams regenerated.')
