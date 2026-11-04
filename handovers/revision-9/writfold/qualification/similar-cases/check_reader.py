"""Contract examples against deliberately arbitrary rankings. No engine or embedding needed."""
import json
from itertools import permutations
from pathlib import Path

def issue(ranked, rows, context, release, limit):
    hits = []
    for key in ranked:
        row = rows.get(key)
        if (not row or not row['admitted'] or not row['witness'] or
                (row['scope'], row['release']) != (context, release)):
            continue
        handle = (row['asset'], row['revision'], row['witness'])
        if handle not in hits and len(hits) < limit:
            hits.append(handle)
    return hits

def main():
    data=json.loads((Path(__file__).parent/'fixtures/pilot.json').read_text())
    rows={c['id']:c for c in data['cases']}
    visible={(r['asset'],r['revision'],r['witness']) for r in rows.values()
             if r['id'] in {'A','B','C'}}
    count=0
    for order in permutations(rows):
        hits=issue(order,rows,'checkout','R42',3)
        assert len(hits)==3 and set(hits)==visible
        count+=1
    target=issue(['A'],rows,'checkout','R42',3)
    assert len(target)==1 and issue([],rows,'checkout','R42',3)==[]
    assert issue(['HIDDEN','OLD','DRAFT','unknown','A','A'],rows,'checkout','R42',3)==target
    assert issue(['A'],rows,'checkout','R42',0)==[]
    print(f'PASS: {count} full rankings, empty, duplicate, missing ID, unauthorized, stale and draft candidates.')
    print('No retrieval quality, token savings or WSL engine performance inferred.')
if __name__=='__main__':
    main()
