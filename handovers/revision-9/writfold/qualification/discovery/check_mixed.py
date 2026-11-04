"""Small cross-kind contract fixture. This is not the Writfold resolver or ranker."""
from dataclasses import dataclass
from itertools import permutations

@dataclass(frozen=True)
class Ref:
    kind: str
    native_id: str
    revision: str

@dataclass(frozen=True)
class Asset:
    ref: Ref
    context: str
    release: str
    role: str

def find(registry, suggestions, eligible_roles, enabled, wanted=None, limit=3):
    picked=[]
    for ref in suggestions:
        asset=registry.get(ref)
        if not asset or (asset.context,asset.release)!=('checkout','R42'):
            continue
        if ref.kind not in enabled or (wanted is not None and ref.kind not in wanted):
            continue
        if asset.role not in eligible_roles or ref in picked:
            continue
        if len(picked) < limit:
            picked.append(ref)
    return picked

def main():
    decision=Ref('decision','17','d3')
    review=Ref('review','17','d3')
    skill=Ref('procedure','2','s7')
    unknown=Ref('incident','17','d3')
    stale=Ref('decision','17','d2')
    private=Ref('review','33','x1')
    disabled=Ref('lesson','4','l1')
    catalog={r:Asset(r,c,p,e) for r,c,p,e in (
        (decision,'checkout','R42','accepted-design'),(review,'checkout','R42','review-comment'),
        (skill,'checkout','R42','guidance'),(stale,'checkout','R41','accepted-design'),
        (private,'private','R42','review-comment'),(disabled,'checkout','R42','guidance'))}
    roles={'accepted-design','review-comment','guidance'}
    qualified={'decision','review','procedure'}
    for order in permutations([decision,review,skill]):
        adversarial=[unknown,private,stale,disabled,*order,review]
        assert find(catalog,adversarial,roles,qualified)==list(order)
        assert find(catalog,adversarial,roles,qualified,{'review'})==[review]
        assert find(catalog,adversarial,roles,qualified,{'incident'})==[]
    assert find(catalog,[review,decision],roles,qualified,limit=1)==[review]
    print('PASS: cross-kind collisions, mixed hits, filters, scope, source cuts, missing profile and bounded output.')
    print('Design fixture only; no native retrieval, proof admission or skill activation.')

if __name__=='__main__':
    main()
