#!/usr/bin/env python3
"""Offline handover integrity checks. Python 3 standard library only.

This validates a planning package, not Writfold's unimplemented product gates.
"""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import urlsplit, unquote
import hashlib
import json
import re
import sys
import tarfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
errors = []
META = json.loads((ROOT / 'registries/bundle.json').read_text())


def check(condition, message):
    if not condition:
        errors.append(message)


def load(name):
    return json.loads((ROOT / 'registries' / (name + '.json')).read_text())


def check_link(source, target):
    url = urlsplit(target)
    if url.scheme or url.netloc or not url.path:
        return
    path = (source.parent / unquote(url.path)).resolve()
    check(path.is_relative_to(ROOT), f'Escaping link: {source.name}: {target}')
    check(path.exists(), f'Missing link: {source.relative_to(ROOT)}: {target}')


class Links(HTMLParser):
    def handle_starttag(self, tag, attrs):
        for key, value in attrs:
            if key in ('href', 'src') and value:
                check_link(ROOT / 'index.html', value)


stages, gates, proofs, scenarios = (load(n) for n in
                                    ('stages', 'gates', 'proofs', 'scenarios'))
collections = [stages, gates, proofs, scenarios]
for records, count in zip(collections, tuple(META[k] for k in ('stage_count','gate_count','proof_count','scenario_count'))):
    ids = [r['id'] for r in records]
    check(len(ids) == count, f'Expected {count} records; got {len(ids)}')
    check(len(ids) == len(set(ids)), 'Duplicate registry identifier')
    check(all(r['status'] == 'not_run' for r in records),
          'Planning registry improperly claims a completed product gate')
check(META.get('selected_backend') == 'omnigraph', 'Incorrect selected backend')
check(META.get('foundation_stage') == 'P01O', 'Missing early foundation stage')
check(any(s['id'] == 'P01O' and 'WF24' in s['gates'] for s in stages), 'Foundation gate missing')
check(any(s['id'] == 'P06' and 'P01O' in s['dependencies'] for s in stages), 'Native adapter bypasses foundation')
check({'V15', 'M11'} <= {p['id'] for p in proofs}, 'Missing storage proof obligations')
check(META.get('revision') == 5, 'Expected revision-5 package')
check(META.get('graph_families') == ['corpus', 'activity'], 'Incorrect graph family decision')
engineering = load('engineering-theorems')
check(len(engineering) == META['engineering_theorem_count'] == 12,
      'Incorrect engineering theorem inventory')
check({r['id'] for r in engineering} == {f'E{i:02}' for i in range(1, 13)},
      'Incomplete E01-E12 targets')
for r in engineering:
    check(r['status'] == 'not_run' and r['receipt'] is None,
          'Unproved engineering target reported complete')
    check((ROOT / r['contract']).is_file(), 'Missing engineering calculus contract')
stage_ids, gate_ids, proof_ids, scenario_ids = (
    {r['id'] for r in records} for records in collections)
by_stage = {r['id']: r for r in stages}
seen = set()
for stage in stages:
    sid = stage['id']
    check(set(stage['dependencies']) <= seen,
          f'{sid}: invalid dependency order or cycle')
    seen.add(sid)
    check(set(stage['gates']) <= gate_ids, f'{sid}: unknown gate')
    check(set(stage['proofs']) <= proof_ids, f'{sid}: unknown proof')
    check(set(stage['scenarios']) <= scenario_ids, f'{sid}: unknown scenario')
    for suffix in ('mmd', 'svg', 'png'):
        check((ROOT / 'diagrams' / f'{sid}.{suffix}').is_file(),
              f'{sid}: missing {suffix} diagram')
    check((ROOT / stage['file']).is_file(), f'{sid}: missing stage instructions')
for records in (gates, scenarios):
    for r in records:
        check(set(r['stages']) <= stage_ids, f"{r['id']}: unknown owner stage")
for gate in gates:
    expected = {s['id'] for s in stages if gate['id'] in s['gates']}
    check(set(gate['stages']) == expected, f"{gate['id']}: gate membership mismatch")
check(next(g for g in gates if g['id'] == 'WF31')['stages'] == ['P11'],
      'Final calculus aggregation must not block its own prerequisite stages')
check({f'V{i}' for i in range(16, 22)} | {f'M{i}' for i in range(12, 17)}
      <= set(by_stage['P11']['proofs']), 'P11 omits engineering proof entries')
check({f'X{i}' for i in range(109, 149)} <= set(by_stage['P11']['scenarios']),
      'P11 omits engineering scenarios')
for r in proofs + scenarios:
    check((ROOT / r['contract']).is_file(), f"{r['id']}: missing contract")
for r in load('legacy-gate-map'):
    check(set(r['writfold_gates']) <= gate_ids, f"{r['id']}: invalid mapping")
check({r['id'] for r in load('legacy-gate-map')} ==
      {f'R{i:02}' for i in range(14)}, 'Incomplete R00-R13 crosswalk')

for path in ROOT.rglob('*.md'):
    if 'reference' in path.relative_to(ROOT).parts:
        continue  # Retained historical documents link into their original repos.
    text = re.sub(r'```.*?```', '', path.read_text(), flags=re.S)
    for target in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', text):
        check_link(path, target)
Links().feed((ROOT / 'index.html').read_text())
for path in ROOT.rglob('*.svg'):
    raw = path.read_text()
    check('<!DOCTYPE' not in raw and '<!ENTITY' not in raw,
          f'{path.name}: external/entity declarations')
    tree = ET.fromstring(raw)
    for element in tree.iter():
        check(element.tag.split('}')[-1] not in ('script', 'foreignObject'),
              f'{path.name}: active SVG content')
        for key, value in element.attrib.items():
            check(not key.lower().startswith('on'), f'{path.name}: SVG event')
            if key.split('}')[-1] in ('href', 'src'):
                check(value.startswith('#'), f'{path.name}: external SVG resource')
    check(bool(tree.attrib.get('viewBox')), f'{path.name}: missing viewBox')

pdf = (ROOT / 'review-atlas.pdf').read_bytes()
check(pdf.startswith(b'%PDF-'), 'Invalid PDF header')
check(len(re.findall(rb'/Type\s*/Page\b', pdf)) == META['atlas_pages'], 'Unexpected atlas page count')
pdf_digest = hashlib.sha256(pdf).hexdigest()
for name in ('atlas-build', 'atlas-review'):
    path = ROOT / 'evidence' / (name + '.json')
    check(path.is_file(), f'Missing {name} record')
    if not path.is_file():
        continue
    record = json.loads(path.read_text())
    check(record.get('pdf_sha256') == pdf_digest,
          f'{name}: receipt does not name the exact current PDF')
    check(record.get('pages') == META['atlas_pages'],
          f'{name}: incorrect page count')
    if name == 'atlas-build':
        inputs = record.get('inputs', {})
        required = {'tools/render_assets.py', 'registries/stages.json',
                    'registries/bundle.json', 'plan/11-workflow3-contract.md',
                    'plan/12-playbook-adoption.md', 'tools/render_view_pages.py',
                    'plan/13-view-channels.md', 'plan/14-obsidian-reading-design.md',
                    'plan/15-storage-decision.md', 'plan/16-omnigraph-storage-contract.md',
                    'tools/render_storage_pages.py'}
        required |= {'tools/render_engineering_pages.py',
                     'registries/engineering-theorems.json',
                     'plan/17-engineering-graphs-and-layout.md',
                     'plan/18-identities-fingerprints-and-cuts.md',
                     'plan/19-skill-packages-and-composition.md',
                     'plan/20-execution-and-cross-repository.md',
                     'plan/21-learning-and-promotion.md',
                     'plan/22-engineering-calculus.md',
                     'plan/23-engineering-extension-acceptance.md'}
        check(required <= set(inputs), 'Atlas build lacks required source identities')
        for relative, digest in inputs.items():
            file = (ROOT / relative).resolve()
            check(file.is_relative_to(ROOT), f'Escaping atlas input: {relative}')
            check(file.is_file(), f'Missing atlas input: {relative}')
            if file.is_file() and file.is_relative_to(ROOT):
                check(hashlib.sha256(file.read_bytes()).hexdigest() == digest,
                      f'Stale atlas input: {relative}')
    else:
        check(record.get('reviewed_pages') == list(range(1, META['atlas_pages'] + 1)),
              'Atlas review does not cover every page')
        check(record.get('outcome') == 'accepted', 'Atlas visual review not accepted')
        check(bool(record.get('method')), 'Atlas review lacks its review method')
archive = ROOT / META['archive']
with tarfile.open(archive, 'r:gz') as tar:
    members = tar.getmembers()
    check(bool(members), 'Empty baseline archive')
    for member in members:
        p = Path(member.name)
        check(not p.is_absolute() and '..' not in p.parts, 'Unsafe archive path')
        check(not member.issym() and not member.islnk(), 'Unexpected archive link')

manifest_path = ROOT / 'MANIFEST.sha256'
check(manifest_path.exists(), 'Missing SHA256 manifest')
listed = set()
if manifest_path.exists():
    for line in manifest_path.read_text().splitlines():
        digest, relative = line.split('  ', 1)
        listed.add(relative)
        file = ROOT / relative
        check(file.is_file(), f'Manifest file missing: {relative}')
        if file.is_file():
            check(hashlib.sha256(file.read_bytes()).hexdigest() == digest,
                  f'Hash mismatch: {relative}')
actual = set()
for file in ROOT.rglob('*'):
    check(not file.is_symlink(), f'Unexpected symlink: {file.name}')
    if file.is_file() and file != manifest_path:
        actual.add(file.relative_to(ROOT).as_posix())
check(actual == listed, 'Manifest file inventory differs from bundle')
if errors:
    print('\n'.join('FAIL: ' + e for e in errors))
    sys.exit(1)
print(f'PASS: {len(actual)} files; {len(stages)} stages; {len(gates)} gates; '
      f'{len(proofs)} proof/model entries; {len(scenarios)} scenarios; '
      f"{META['atlas_pages']} PDF pages; links and vector assets; SHA256 integrity.")
print('This result validates the handover package. Product gates remain not_run.')
