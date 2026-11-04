"""Kernel-checked concrete counterexamples and rejection of mutated safety proofs."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import tempfile

from writfold_conformance_process import Commands, record, sha, write_json

PREFIX = '''open Kctx Kctx.Citation

def sourceRef : RepresentationRef :=
  ⟨⟨1⟩, ⟨"repo", by decide⟩, ⟨"revision", by decide⟩, "file", ⟨"blob", by decide⟩, .gitBlob⟩
def citeA : Binding :=
  ⟨⟨"cite", by decide⟩, ⟨"release", by decide⟩, ⟨"context", by decide⟩,
    sourceRef, ⟨0, 1, .byte⟩, [97]⟩
def objectA : Representation := ⟨sourceRef, [97]⟩
def storeA : Store := [objectA]
def yes : Access := fun _ => true
'''
ALIAS = '''
def scopeA : AliasScope :=
  ⟨⟨1⟩, ⟨"actor", by decide⟩, ⟨1⟩, ⟨"release", by decide⟩, ⟨"context", by decide⟩, 1⟩
def tableA : AliasTable := ⟨scopeA, [⟨⟨1⟩, citeA, 0, 10⟩]⟩
'''
PAGE = ALIAS + '''
def queryA : NoticeQuery :=
  ⟨scopeA, ⟨"query", by decide⟩, ⟨"order", by decide⟩, 1,
    [⟨.evidence, "retained notice"⟩], .measured 1 10⟩
'''
REUSE = '''
def attack : Bool :=
  let reference := { sourceRef with
    repository := NonVacuity.observation.source
    revision := NonVacuity.observation.revision
    content := NonVacuity.observation.content }
  let original := { citeA with representation := reference }
  let changed := { original with id := ⟨"different-binding", by decide⟩ }
  let derivation : Derivation :=
    ⟨⟨"derivation", by decide⟩, NonVacuity.claim.ref, .attest NonVacuity.attestation.id⟩
  match checkedDerivation NonVacuity.kb NonVacuity.context derivation with
  | none => false
  | some checked =>
    let receipt : Receipt := {
      id := ⟨"receipt", by decide⟩
      claim := NonVacuity.claim.ref
      context := NonVacuity.context
      witness := derivation.witness
      dependencies := checked.dependencies
      passed := true
      expiresAt := 30 }
    let validation : BindingValidation := ⟨receipt, original.context, [original]⟩
    citationReusable [⟨reference, [97]⟩] yes NonVacuity.kb NonVacuity.context
      original.context NonVacuity.claim.ref [changed] validation
'''
RETIME = '''
def attack : Bool :=
  let state : Kctx.Citation.State := {
    engineering := { (default : Engineering.State) with
      knowledge := { (default : PortfolioState) with kb := NonVacuity.kb } }
    citations := default }
  match localStep yes state (.acquisitionHint sourceRef 999) with
  | none => false
  | some after => decide
      (after.engineering.knowledge.kb.observations ≠ state.engineering.knowledge.kb.observations)
'''


@dataclass(frozen=True)
class Mutation:
    name: str
    model: str
    proof: str
    before: str
    after: str
    witness: str
    expected_declarations: tuple[str, ...]
    imports: tuple[str, ...] = ()


def mutations() -> tuple[Mutation, ...]:
    return (
        Mutation('ignore-selector-bounds', 'Resolution', 'ResolutionProofs',
                 'if binding.selector.first ≤ binding.selector.last ∧\n            binding.selector.last ≤ object.bytes.length then',
                 'if True then',
                 '''def attack : Bool := match resolve storeA yes
    { citeA with selector := ⟨3, 3, .byte⟩, expected := [] } with
  | .exact _ _ => true
  | _ => false
''', ('resolve_exact',)),
        Mutation('ignore-selection-preimage', 'Resolution', 'ResolutionProofs',
                 'if selectedBytes object.bytes binding.selector = binding.expected then',
                 'if True then',
                 '''def attack : Bool := match resolve storeA yes { citeA with expected := [98] } with
  | .exact _ _ => true
  | _ => false
''', ('resolve_exact',)),
        Mutation('replace-retained-history', 'Resolution', 'ResolutionProofs',
                 '| some _ => none', '| some _ => some (object :: store)',
                 '''def attack : Bool :=
  let replacement := { objectA with bytes := [98] }
  match retain storeA replacement with
  | none => false
  | some next => decide (lookupRepresentation next sourceRef = some replacement)
''', ('retain_rejects_existing', 'retain_preserves_lookup')),
        Mutation('ignore-current-access', 'Handles', 'HandlesProofs',
                 'else if access entry.binding = false then .denied',
                 'else if False then .denied',
                 ALIAS + '''def attack : Bool := match expandAlias tableA scopeA ⟨1⟩ 5 (fun _ => false) with
  | .durable _ => true
  | _ => false
''', ('expandAlias_exact_agreement', 'expandAlias_revoked_denied')),
        Mutation('ignore-alias-audience', 'Handles', 'HandlesProofs',
                 'if sameAliasAudience table.scope scope = false then .foreign',
                 'if False then .foreign',
                 ALIAS + '''def attack : Bool := match expandAlias tableA { scopeA with realm := ⟨2⟩ } ⟨1⟩ 5 yes with
  | .durable _ => true
  | _ => false
''', ('expandAlias_exact_agreement', 'expandAlias_foreign_refused')),
        Mutation('accept-foreign-cursor', 'Presentation', 'PresentationProofs',
                 'if continuation.query = query then', 'if True then',
                 PAGE + '''def attack : Bool := match presentNotices queryA
    (some ⟨{ queryA with scope := { scopeA with realm := ⟨2⟩ } }, 0⟩) with
  | .ok _ => true
  | .error _ => false
''', ('presentNotices_cursor_pinned', 'presentNotices_mismatched_cursor_refused')),
        Mutation('promote-partial-coverage', 'Presentation', 'PresentationProofs',
                 'coverage := query.coverage', 'coverage := .measured 0 0',
                 PAGE + '''def attack : Bool := match presentNotices queryA none with
  | .ok page => page.coverage.complete
  | .error _ => false
''', ('presentNotices_coverage_unchanged',)),
        Mutation('reuse-changed-binding', 'Reuse', 'ReuseProofs',
                 'decide (current = validation.bindings)', 'true', REUSE,
                 ('CIT05_reuse_requires_exact_bindings', 'CIT05_changed_binding_refuses'),
                 ('Kctx.NonVacuity',)),
        Mutation('hint-retimestamps-observations', 'State', 'Conservation',
                 '(fun data => { state with citations := data })',
                 '''(fun data => { state with
    citations := data
    engineering := { state.engineering with
      knowledge := { state.engineering.knowledge with
        kb := { state.engineering.knowledge.kb with
          observations := state.engineering.knowledge.kb.observations.map
            (fun observation => { observation with observedAt := observation.observedAt + 1 }) } } } })''',
                 RETIME, ('localStep_engineering_frame',), ('Kctx.NonVacuity',)),
    )


def fork_library(base: Path, destination: Path, omitted: set[Path]) -> None:
    for path in base.rglob('*'):
        if not path.is_file():
            continue
        relative = path.relative_to(base)
        if relative in omitted:
            continue
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.symlink_to(path.resolve())


def failed_declarations(proof: Path, text: str) -> set[str]:
    declarations = [(number, match[1]) for number, line in enumerate(proof.read_text().splitlines(), 1)
                    if (match := re.match(r'(?:private )?theorem\s+(\w+)', line))]
    found = set()
    for match in re.finditer(re.escape(str(proof)) + r':(\d+):\d+: error:', text):
        previous = [name for number, name in declarations if number <= int(match[1])]
        if previous:
            found.add(previous[-1])
    return found


def retain_source(source: Path, destination: Path) -> dict:
    with destination.open('xb') as stream:
        stream.write(source.read_bytes())
    retained = record(destination)
    if retained['sha256'] != sha(source):
        raise ValueError('Calibration source changed while retaining')
    return {**retained, 'compiled_path': str(source)}


def attempt(commands: Commands, work: Path, output: Path, mutation: Mutation,
            *, changed: bool) -> dict:
    label = mutation.name + ('-mutant' if changed else '-control')
    retained = output / 'calibration' / label
    retained.mkdir(parents=True, exist_ok=False)
    root = Path(tempfile.mkdtemp(prefix='citation-calibration-', dir=work.parent))
    src, lib = root / 'src', root / 'lib'
    src.mkdir()
    lib.mkdir()
    relative_model = Path('Kctx/Citation') / (mutation.model + '.lean')
    relative_proof = Path('Kctx/Citation') / (mutation.proof + '.lean')
    fork_library(work / '.lake/build/lib/lean', lib,
                 {relative_model.with_suffix('.olean'), relative_proof.with_suffix('.olean')})
    model, proof = src / relative_model, src / relative_proof
    model.parent.mkdir(parents=True)
    original = (work / relative_model).read_text()
    if original.count(mutation.before) != 1:
        raise ValueError('Mutation no longer selects one declared body: ' + mutation.name)
    model.write_text(original.replace(mutation.before, mutation.after) if changed else original)
    proof.write_bytes((work / relative_proof).read_bytes())
    model_output, proof_output = lib / relative_model.with_suffix('.olean'), lib / relative_proof.with_suffix('.olean')
    model_output.parent.mkdir(parents=True, exist_ok=True)
    environment = {'LEAN_PATH': str(lib)}
    result = {'name': label, 'scope': 'actual source mutation with kernel-checked counterexample',
              'changed': changed, 'model': retain_source(model, retained / 'model.lean'),
              'proof': retain_source(proof, retained / 'proof.lean'), 'work': str(root)}
    commands.run(label + '-model', ['lean', '-R', str(src), '-o', str(model_output), str(model)],
                 cwd=src, environment=environment, timeout=300)
    witness = src / 'Witness.lean'
    imports = ('Kctx.Citation.' + mutation.model, *mutation.imports)
    expected = 'true' if changed else 'false'
    witness.write_text(''.join('import ' + name + '\n' for name in imports) + PREFIX + mutation.witness +
                       '\nexample : attack = ' + expected + ' := by decide\n#eval attack\n')
    result['witness'] = retain_source(witness, retained / 'witness.lean')
    observed = commands.run(label + '-witness', ['lean', str(witness)], cwd=src,
                            environment=environment, timeout=300)
    if observed.read_text().strip() != expected:
        raise ValueError('Counterexample did not execute as expected: ' + label)
    result['counterexample_observed'] = expected
    try:
        commands.run(label + '-proof', ['lean', '-R', str(src), '-o', str(proof_output), str(proof)],
                     cwd=src, environment=environment, timeout=300)
    except ValueError:
        last = commands.records[-1]
        text = '\n'.join(Path(log['path']).read_text(errors='replace') for log in last['logs'])
        if (not changed or last['exit_code'] != 1 or last['timed_out'] or
                any(marker in text for marker in ('unexpected token', 'unknown identifier',
                    'unknown module', 'does not exist', 'failed to synthesize'))):
            raise ValueError('Nonsemantic calibration failure: ' + label)
        rejected = failed_declarations(proof, text)
        if not rejected.intersection(mutation.expected_declarations):
            raise ValueError('Mutation missed its intended safety declaration: ' + label)
        result['rejected_declarations'] = sorted(rejected)
        result['proof_rejection_command'] = len(commands.records) - 1
    else:
        if changed:
            raise ValueError('Safety proof accepted a behavior-changing mutation: ' + label)
        result['rejected_declarations'] = []
    result['result'] = 'pass'
    return result


def calibrate(commands: Commands, work: Path, output: Path) -> dict:
    report = {'scope': 'semantic source mutations; not production Rust or IO qualification',
              'controls': [], 'mutants': [], 'result': 'fail'}
    try:
        for mutation in mutations():
            report['controls'].append(attempt(commands, work, output, mutation, changed=False))
            report['mutants'].append(attempt(commands, work, output, mutation, changed=True))
        report['result'] = 'pass'
    finally:
        write_json(output / 'citation-mutation-report.json', report)
    return report
