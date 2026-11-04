#!/usr/bin/env python3
"""Offline specimen checks; not Obsidian rendering or plugin execution."""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
VAULT = ROOT / 'examples/obsidian-vault'
manifest = json.loads((VAULT / 'sample-manifest.json').read_text())
prefix = 'Writfold/Generations/' + manifest['generation'] + '/'
notes = {}
links = 0
for relative, digest in manifest['files'].items():
    path = VAULT / relative
    assert path.resolve().is_relative_to(VAULT.resolve()), relative
    assert hashlib.sha256(path.read_bytes()).hexdigest() == digest, relative
    text = path.read_text()
    if relative.endswith('.md') and relative.startswith(prefix):
        front = text.split('---', 2)[1]
        props = {k: json.loads(v) for k, v in
                 re.findall(r'^(wf_\w+): (.+)$', front, flags=re.M)}
        assert props['wf_generation'] == manifest['generation']
        assert props['wf_context'] == manifest['context']
        assert props['wf_id'] not in notes
        assert props['wf_title']
        notes[props['wf_id']] = props
    for target in re.findall(r'\[\[([^\]]+)\]\]', text):
        name = target.split('|', 1)[0].split('#', 1)[0]
        target_path = VAULT / name
        if not target_path.suffix:
            target_path = target_path.with_suffix('.md')
        assert target_path.resolve().is_relative_to(VAULT.resolve()), target
        assert target_path.is_file(), (relative, target)
        assert name.startswith(prefix) or name.startswith('Human/'), target
        links += 1
    assert '```dataviewjs' not in text and '$=' not in text

assert sorted(k for k,v in notes.items() if v['wf_kind']=='service') == manifest['expected']['services']
assert sorted(k for k,v in notes.items() if v['wf_kind']=='service' and v['wf_status']=='needs-evidence') == manifest['expected']['attention']
assert sorted(k for k,v in notes.items() if v['wf_kind']=='evidence') == manifest['expected']['evidence']
base = (VAULT / prefix / 'Assets.base').read_text()
for required in ['file.inFolder("'+prefix+'Services")', 'wf_generation == "demo-r1"',
                 'wf_context == "demo-operations"', 'wf_kind == "service"']:
    assert required in base
dashboard = (VAULT / prefix / 'Dashboards.md').read_text()
queries = re.findall(r'```dataview\n(.*?)```', dashboard, re.S)
assert len(queries) == 2
for query in queries:
    for required in ['FROM "'+prefix, 'wf_context = "demo-operations"',
                     'wf_generation = "demo-r1"', 'LIMIT 25']:
        assert required in query
profile = json.loads((VAULT / 'plugin-profile.json').read_text())
assert profile['release_tag'] == profile['manifest_version'] == '0.5.68'
assert len(profile['assets']) == 3
assert all(re.fullmatch('[0-9a-f]{64}',a['sha256']) for a in profile['assets'])
print(f'PASS: {len(notes)} generation notes, {links} links, one scoped Base, two scoped DQL templates; manifest and expected IDs.')
print('Static checks only. Native Obsidian/CLI/plugin and visual behavior remain WF23 not_run.')
