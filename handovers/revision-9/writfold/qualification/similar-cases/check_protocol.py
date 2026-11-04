"""Exhaustive finite reference model for the actions in formal/IndexPublication.tla."""
from collections import deque
from dataclasses import dataclass, replace
import json

@dataclass(frozen=True)
class State:
    desired: int
    pending: int = 0
    active: int = 0
    reader: int = 0
    handle: int = 0
    grant: bool = True
    pulse: bool = False

def edges(s, mutant):
    for epoch in (1, 2):
        yield 'Target'+str(epoch), replace(s, desired=epoch)
    yield 'Build', replace(s, pending=s.desired)
    if s.pending and (s.pending == s.desired or mutant == 'publish'):
        yield 'Publish', replace(s, pending=0, active=s.pending)
    yield 'Attach', replace(s, reader=s.active)
    yield 'Toggle', replace(s, grant=not s.grant)
    if s.reader and s.grant:
        yield 'Issue', replace(s, handle=s.reader)
    if s.reader and s.handle and (s.grant or mutant == 'revoke') and (s.handle == s.reader or mutant == 'epoch'):
        yield 'Open', replace(s, pulse=not s.pulse)

def violation(s, t):
    if s.active != t.active and t.active != s.desired:
        return 'FreshPublish'
    if t.pulse != s.pulse and not (s.grant and s.reader and s.reader == s.handle):
        return 'CheckedOpen'
    if t.handle != s.handle and not (t.handle == s.reader and s.reader and s.grant):
        return 'EligibleIssue'
    return None

def check(mutant):
    q=deque(State(i) for i in (1,2))
    predecessors={s: None for s in q}
    transitions=0
    while q:
        s=q.popleft()
        for action,t in edges(s,mutant):
            transitions += 1
            failed=violation(s,t)
            if failed:
                trace=[action]
                while predecessors[s]:
                    s, previous=predecessors[s]
                    trace.append(previous)
                trace.reverse()
                return {'violation':failed,'initial_target':s.desired,'trace':trace}
            if t not in predecessors:
                predecessors[t]=(s,action)
                q.append(t)
    return {'states':len(predecessors),'transitions':transitions,'violation':None}

if __name__ == '__main__':
    clean=check(None)
    assert clean['violation'] is None
    runs={mutant:check(mutant) for mutant in ('publish','revoke','epoch')}
    assert all(run.get('violation') for run in runs.values())
    print(json.dumps({'reference':clean,'mutation_counterexamples':runs},indent=2))
