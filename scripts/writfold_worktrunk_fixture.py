"""Disposable native fixtures only; the static Rust consumer performs adapter IO."""
import hashlib
import json
import os
from pathlib import Path
import sys

sys.path.insert(0, '/work/writfold/scripts')
from writfold_omnigraph_companion import Process

root = Path('/evidence')
fixture = root / 'fixture'
git = '/usr/bin/git'
wt = root / 'wt'
consumer = root / 'worktrunk_smoke'
expected_git = 'eb54b535467e0975c1284d4c4d08eea779aeb248a5d34b02e2ee171382e22201'
expected_wt = '1f3d97a5c263f87345d03a1e439f5d53efa5a468e8bde10e724cbd9e5e79de9b'
if not __debug__:
    raise RuntimeError('qualification assertions require ordinary Python execution')
if len(sys.argv) > 2:
    raise ValueError('expected at most one corruption-control argument')
commands = []
checks = []
record = {'scope': 'Actual Rust Worktrunk adapter, native SHA1/SHA256 creation and armed hook controls; no owner service or alpha admission',
          'status': 'fail', 'commands': commands, 'checks': checks, 'uid': os.getuid(), 'gid': os.getgid()}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def run(name, argv, environment, timeout=60):
    process = Process(root, name, [str(value) for value in argv], environment, [], commands)
    outcome = process.finish(timeout)
    assert outcome['exit'] == 0, (name, outcome)
    assert outcome['reaped'] and outcome['group_absent'] and outcome['logs_drained']
    assert not outcome['forced_kill']
    assert all(not value.get('error') and not value['truncated'] for value in outcome['logs'].values())
    return (root / (name + '.stdout')).read_bytes()


def hook_text(marker):
    return 'printf armed > ' + str(marker)


try:
    assert sha(git) == expected_git and sha(wt) == expected_wt
    record['executables'] = {name: {'path': str(path), 'sha256': sha(path)}
                             for name, path in [('git', git), ('worktrunk', wt), ('consumer', consumer)]}
    fixture.mkdir(mode=0o700)
    home = root / 'fixture-home'
    home.mkdir(mode=0o700)
    empty = root / 'empty.toml'
    empty.write_bytes(b'')
    environment = {'PATH': '/usr/bin:/bin', 'HOME': str(home), 'XDG_CONFIG_HOME': str(home),
                   'LC_ALL': 'C', 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_SYSTEM': '/dev/null',
                   'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_NO_REPLACE_OBJECTS': '1',
                   'GIT_TERMINAL_PROMPT': '0', 'GIT_AUTHOR_NAME': 'Native fixture',
                   'GIT_AUTHOR_EMAIL': 'fixture@example.invalid', 'GIT_COMMITTER_NAME': 'Native fixture',
                   'GIT_COMMITTER_EMAIL': 'fixture@example.invalid'}
    for name in ('sha1', 'sha256'):
        directory = fixture / name
        directory.mkdir(mode=0o700)
        repo = directory / 'repo'
        repo.mkdir(mode=0o700)
        run(name + '-init', [git, 'init', '--template=', '--initial-branch=main', '--object-format=' + name, repo], environment)
        (repo / '.git').chmod(0o700)
        (repo / '.git/writfold-locks').mkdir(mode=0o700)
        destinations = directory / 'destinations'
        destinations.mkdir(mode=0o700)
        (destinations / 'writfold-locks').mkdir(mode=0o700)
        (repo / 'file').write_bytes(b'exact base\0\n')
        (repo / 'nested').mkdir(mode=0o700)
        (repo / 'nested/copy').write_bytes(b'exact base\0\n')
        (repo / 'nested/empty').write_bytes(b'')
        run(name + '-add-base', [git, '-C', repo, 'add', '--', 'file', 'nested'], environment)
        run(name + '-commit-base', [git, '-C', repo, 'commit', '--no-gpg-sign', '-m', 'base'], environment)
        base = run(name + '-base', [git, '-C', repo, 'rev-parse', 'HEAD'], environment)
        (directory / 'base').write_bytes(base)
        (repo / 'file').write_bytes(b'later source\0\n')
        run(name + '-add-later', [git, '-C', repo, 'add', '--', 'file'], environment)
        run(name + '-commit-later', [git, '-C', repo, 'commit', '--no-gpg-sign', '-m', 'later'], environment)
        (repo / 'untracked').write_bytes(b'preserve unrelated original bytes\0\n')
        hooks = repo / '.git/hooks'
        hooks.mkdir(mode=0o700)
        marker = directory / 'git-marker'
        hook = hooks / 'post-checkout'
        hook.write_text('#!/bin/sh\n' + hook_text(marker) + '\n')
        hook.chmod(0o700)
        run(name + '-armed-git-base', [git, '-C', repo, 'checkout', '--detach', base.decode().strip()], environment)
        assert marker.read_bytes() == b'armed'
        run(name + '-restore-main', [git, '-C', repo, 'checkout', 'main'], environment)
        marker.unlink()
        checks.append(name + ': real Git hook positive control; original main restored')

    user = root / 'hostile-user.toml'
    system = root / 'hostile-system.toml'
    user_pre = '; '.join(hook_text(fixture / name / 'user-marker') for name in ('sha1', 'sha256'))
    user_post = '; '.join(hook_text(fixture / name / 'background-marker') for name in ('sha1', 'sha256'))
    system_pre = '; '.join(hook_text(fixture / name / 'system-marker') for name in ('sha1', 'sha256'))
    user.write_text('pre-start = ' + json.dumps(user_pre) + '\npost-start = ' + json.dumps(user_post) + '\n')
    system.write_text('pre-start = ' + json.dumps(system_pre) + '\n')
    wt_env = {**environment, 'WORKTRUNK_SYSTEM_CONFIG_PATH': str(empty), 'WORKTRUNK_PROJECT_CONFIG_PATH': ''}
    wt_prefix = [wt, '--config', user, '-C', fixture / 'sha1/repo', 'hook']
    run('armed-user-pre', wt_prefix + ['pre-start', '--foreground'], wt_env)
    run('armed-user-post', wt_prefix + ['post-start', '--foreground'], wt_env)
    run('armed-system-pre', [wt, '--config', empty, '-C', fixture / 'sha1/repo', 'hook', 'pre-start', '--foreground'],
        {**wt_env, 'WORKTRUNK_SYSTEM_CONFIG_PATH': str(system)})
    for name in ('sha1', 'sha256'):
        for marker_name in ('user-marker', 'background-marker', 'system-marker'):
            marker = fixture / name / marker_name
            assert marker.read_bytes() == b'armed'
            marker.unlink()
    checks.append('real Worktrunk user pre/post-start and system pre-start positive controls')

    fake_path = root / 'hostile-path'
    fake_path.mkdir(mode=0o700)
    fake_git = fake_path / 'git'
    fake_git.write_text('#!/bin/sh\n' + '; '.join(hook_text(fixture / name / 'ambient-git-marker')
                        for name in ('sha1', 'sha256')) + '\nexit 77\n')
    fake_git.chmod(0o700)
    poisoned = {**environment, 'PATH': str(fake_path), 'WORKTRUNK_CONFIG_PATH': str(user),
                'WORKTRUNK_SYSTEM_CONFIG_PATH': str(system),
                'WORKTRUNK_PROJECT_CONFIG_PATH': str(user)}
    corruption = sys.argv[1] if len(sys.argv) == 2 else None
    if corruption not in (None, 'blob', 'tree', 'commit'):
        raise ValueError('unknown native corruption control')
    if corruption is not None:
        for name in ('sha1', 'sha256'):
            repo = fixture / name / 'repo'
            base = (fixture / name / 'base').read_text().strip()
            suffix = {'blob': ':file', 'tree': '^{tree}', 'commit': ''}[corruption]
            original_oid = run(name + '-original-object',
                               [git, '-C', repo, 'rev-parse', base + suffix], environment).decode().strip()
            replacement_oid = run(name + '-replacement-object',
                                  [git, '-C', repo, 'rev-parse', 'HEAD' + suffix], environment).decode().strip()
            assert original_oid != replacement_oid
            original_object = repo / '.git/objects' / original_oid[:2] / original_oid[2:]
            replacement_object = repo / '.git/objects' / replacement_oid[:2] / replacement_oid[2:]
            before = sha(original_object)
            replacement = replacement_object.read_bytes()
            original_object.chmod(0o600)
            original_object.write_bytes(replacement)
            original_object.chmod(0o444)
            record.setdefault('corruptions', []).append({
                'format': name, 'kind': corruption, 'original_oid': original_oid,
                'replacement_oid': replacement_oid, 'compressed_before_sha256': before,
                'compressed_after_sha256': sha(original_object)})
        argv = [consumer, git, expected_git, wt, fixture, '--expect-corrupt-refusal']
        expected = b'SHA1/SHA256 corrupt base closure refused before creation'
    else:
        argv = [consumer, git, expected_git, wt, fixture]
        expected = b'SHA1/SHA256 native bindings, preservation, conflicts, pins and armed-control nonexecution observed'
    output = run('rust-adapter', argv, poisoned, timeout=180)
    assert expected in output
    for name in ('sha1', 'sha256'):
        assert (fixture / name / 'repo/untracked').read_bytes() == b'preserve unrelated original bytes\0\n'
        assert (fixture / name / 'repo/file').read_bytes() == b'later source\0\n'
    checks.extend(['static consumer exercised exact native binding and refusal assertions',
                   'original tracked and untracked bytes remain exact'])
    record.update(status='pass',consumer_stdout_sha256=hashlib.sha256(output).hexdigest())
finally:
    (root / 'native-results.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({'status': record['status'], 'checks': checks}))
