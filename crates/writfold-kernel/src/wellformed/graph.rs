//! Supersession acyclicity uses the retained borrowed-key DFS.
//! Reachability is the intersection of forward-closed sets, independent of DFS.
use super::{index, order::Key};
#[cfg(verus_keep_ghost)]
use super::{index::visited, sort::sorted, spec::correction_wf_spec};
use vstd::prelude::*;
use writfold_domain::*;

verus! {
broadcast use vstd::seq_lib::group_seq_properties;
#[cfg(verus_keep_ghost)]
pub type Vertex = (Seq<char>, Seq<char>);
pub open spec fn reference_key(reference: ClaimRefView) -> Vertex { (reference.id, reference.rev) }
pub open spec fn vertices(kb: KBView) -> Set<Vertex> {
    kb.claims.map_values(|c: ClaimView| (c.id, c.revision)).to_set()
}
pub open spec fn row_edge(row: CorrectionView, from: Vertex, to: Vertex) -> bool {
    row.kind == CorrectionKind::Supersede && reference_key(row.target) == from
        && match row.replacement { Some(next) => reference_key(next) == to, None => false }
}
pub open spec fn edge_prefix(kb: KBView, count: int, from: Vertex, to: Vertex) -> bool {
    exists|i: int| 0 <= i < count && row_edge(kb.corrections[i], from, to)
}
pub open spec fn edge(kb: KBView, from: Vertex, to: Vertex) -> bool {
    edge_prefix(kb, kb.corrections.len() as int, from, to)
}
pub open spec fn closed(kb: KBView, set: Set<Vertex>) -> bool {
    forall|from: Vertex, to: Vertex| set.contains(from) && edge(kb, from, to) ==> set.contains(to)
}
pub open spec fn reachable(kb: KBView, start: Vertex, to: Vertex) -> bool {
    forall|set: Set<Vertex>| closed(kb, set) && set.contains(start) ==> set.contains(to)
}
pub open spec fn no_cycle_from(kb: KBView, start: Vertex) -> bool {
    forall|from: Vertex| reachable(kb, start, from) ==> !edge(kb, from, start)
}
pub open spec fn acyclic(kb: KBView) -> bool {
    forall|i: int| 0 <= i < kb.claims.len() ==> no_cycle_from(kb, (kb.claims[i].id, kb.claims[i].revision))
}
proof fn reachable_step(kb: KBView, start: Vertex, from: Vertex, to: Vertex)
    requires reachable(kb, start, from), edge(kb, from, to),
    ensures reachable(kb, start, to),
{
    assert forall|set: Set<Vertex>| closed(kb, set) && set.contains(start) implies set.contains(to) by {
        assert(set.contains(from));
    }
}
proof fn vertices_closed(kb: KBView)
    requires forall|i: int| 0 <= i < kb.corrections.len() ==> correction_wf_spec(kb.corrections[i], kb),
    ensures closed(kb, vertices(kb)),
{
    assert forall|from: Vertex, to: Vertex| vertices(kb).contains(from) && edge(kb, from, to)
        implies vertices(kb).contains(to) by {
        let i = choose|i: int| 0 <= i < kb.corrections.len() && row_edge(kb.corrections[i], from, to);
        let replacement = kb.corrections[i].replacement.unwrap();
        let j = choose|j: int| 0 <= j < kb.claims.len() && claim_matches_spec(kb.claims[j], replacement);
        assert(kb.claims.map_values(|c: ClaimView| (c.id, c.revision))[j] == to);
    }
}
spec fn frontier_reachable(kb: KBView, start: Vertex, frontier: Seq<Vertex>) -> bool {
    forall|i: int| 0 <= i < frontier.len() ==> reachable(kb, start, frontier[i])
}
spec fn processed(kb: KBView, start: Vertex, seen: Set<Vertex>, frontier: Seq<Vertex>) -> bool {
    forall|from: Vertex| seen.contains(from) ==> !edge(kb, from, start)
        && (forall|to: Vertex| edge(kb, from, to) ==> seen.contains(to) || frontier.contains(to))
}
proof fn processed_pop(kb: KBView, start: Vertex, seen: Set<Vertex>, rest: Seq<Vertex>, current: Vertex)
    requires processed(kb, start, seen, rest.push(current)), seen.contains(current),
    ensures processed(kb, start, seen, rest),
{
    assert forall|from: Vertex| seen.contains(from) implies !edge(kb, from, start)
        && (forall|to: Vertex| edge(kb, from, to) ==> seen.contains(to) || rest.contains(to)) by {
        assert forall|to: Vertex| edge(kb, from, to) implies seen.contains(to) || rest.contains(to) by {
            assert(seen.contains(to) || rest.push(current).contains(to));
        }
    }
}
proof fn processed_insert(kb: KBView, start: Vertex, seen: Set<Vertex>, rest: Seq<Vertex>, current: Vertex, expanded: Seq<Vertex>)
    requires processed(kb, start, seen, rest.push(current)), !edge(kb, current, start),
        forall|to: Vertex| rest.contains(to) ==> expanded.contains(to),
        forall|to: Vertex| edge(kb, current, to) ==> expanded.contains(to),
    ensures processed(kb, start, seen.insert(current), expanded),
{
    assert forall|from: Vertex| seen.insert(current).contains(from) implies !edge(kb, from, start)
        && (forall|to: Vertex| edge(kb, from, to) ==> seen.insert(current).contains(to) || expanded.contains(to)) by {
        assert forall|to: Vertex| edge(kb, from, to)
            implies seen.insert(current).contains(to) || expanded.contains(to) by {
            if from != current {
                assert(seen.contains(to) || rest.push(current).contains(to));
            }
        }
    }
}
fn extend_frontier<'a>(kb: &'a KB, start: Key<'a>, current: Key<'a>, frontier: &mut Vec<Key<'a>>) -> (valid: bool)
    requires reachable(kb@, start.deep_view(), current.deep_view()),
        frontier_reachable(kb@, start.deep_view(), old(frontier).deep_view()),
    ensures valid == !edge(kb@, current.deep_view(), start.deep_view()),
        valid ==> frontier_reachable(kb@, start.deep_view(), final(frontier).deep_view()),
        valid ==> (forall|to: Vertex| old(frontier).deep_view().contains(to) ==> final(frontier).deep_view().contains(to)),
        valid ==> (forall|to: Vertex| edge(kb@, current.deep_view(), to) ==> final(frontier).deep_view().contains(to)),
{
    let rows = kb.corrections();
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(), rows.deep_view() == kb@.corrections,
            rows.len() == kb@.corrections.len(),
            reachable(kb@, start.deep_view(), current.deep_view()),
            frontier_reachable(kb@, start.deep_view(), frontier.deep_view()),
            !edge_prefix(kb@, i as int, current.deep_view(), start.deep_view()),
            forall|to: Vertex| old(frontier).deep_view().contains(to) ==> frontier.deep_view().contains(to),
            forall|to: Vertex| edge_prefix(kb@, i as int, current.deep_view(), to) ==> frontier.deep_view().contains(to),
        decreases rows.len() - i,
    {
        let correction = &rows[i];
        if *correction.kind() == CorrectionKind::Supersede
            && (correction.target().id().as_str(), correction.target().rev().as_str()) == current {
            let Some(replacement) = correction.replacement() else {
                i += 1;
                continue;
            };
            let next = (replacement.id().as_str(), replacement.rev().as_str());
            proof {
                assert(row_edge(kb@.corrections[i as int], current.deep_view(), next.deep_view()));
                reachable_step(kb@, start.deep_view(), current.deep_view(), next.deep_view());
            }
            if next == start { return false; }
            let ghost previous = frontier.deep_view();
            frontier.push(next);
            proof {
                assert(frontier.deep_view() =~= previous.push(next.deep_view()));
                assert forall|j: int| 0 <= j < frontier.len()
                    implies reachable(kb@, start.deep_view(), frontier.deep_view()[j]) by {
                    if j < previous.len() { assert(frontier.deep_view()[j] == previous[j]); }
                }
            }
        }
        i += 1;
    }
    true
}
proof fn completed_search(kb: KBView, start: Vertex, seen: Set<Vertex>)
    requires seen.contains(start), processed(kb, start, seen, Seq::empty()),
    ensures no_cycle_from(kb, start),
{
    assert forall|from: Vertex, to: Vertex| seen.contains(from) && edge(kb, from, to)
        implies seen.contains(to) by {
        assert(seen.contains(to) || Seq::<Vertex>::empty().contains(to));
    }
    assert(closed(kb, seen));
    assert forall|from: Vertex| reachable(kb, start, from) implies !edge(kb, from, start) by {
        assert(seen.contains(from));
    }
}
fn search_start<'a>(kb: &'a KB, start: Key<'a>, keys: &[Key<'_>], generation: usize, frontier: &mut Vec<Key<'a>>, seen: &mut Vec<usize>) -> (valid: bool)
    requires vertices(kb@).contains(start.deep_view()), sorted(keys.deep_view()),
        keys.deep_view().no_duplicates(), keys.deep_view().to_set() == vertices(kb@),
        old(seen).len() == keys.len(),
        generation > 0, forall|i: int| 0 <= i < old(seen).len() ==> old(seen)@[i] < generation,
        forall|i: int| 0 <= i < kb@.corrections.len() ==> correction_wf_spec(kb@.corrections[i], kb@),
    ensures valid == no_cycle_from(kb@, start.deep_view()), final(seen).len() == old(seen).len(),
        forall|i: int| 0 <= i < final(seen).len() ==> final(seen)@[i] <= generation,
{
    frontier.clear();
    frontier.push(start);
    proof {
        assert(frontier.deep_view() =~= seq![start.deep_view()]);
        vertices_closed(kb@);
    }
    while !frontier.is_empty()
        invariant
            sorted(keys.deep_view()), keys.deep_view().no_duplicates(),
            keys.deep_view().to_set() == vertices(kb@), seen.len() == keys.len(), seen.len() == old(seen).len(),
            generation > 0, forall|i: int| 0 <= i < seen.len() ==> seen@[i] <= generation,
            vertices(kb@).contains(start.deep_view()), closed(kb@, vertices(kb@)),
            frontier_reachable(kb@, start.deep_view(), frontier.deep_view()),
            forall|v: Vertex| visited(keys.deep_view(), seen@, generation).contains(v) ==> reachable(kb@, start.deep_view(), v),
            visited(keys.deep_view(), seen@, generation).subset_of(vertices(kb@)),
            visited(keys.deep_view(), seen@, generation).contains(start.deep_view()) || frontier.deep_view().contains(start.deep_view()),
            processed(kb@, start.deep_view(), visited(keys.deep_view(), seen@, generation), frontier.deep_view()),
        decreases vertices(kb@).difference(visited(keys.deep_view(), seen@, generation)).len(), frontier.len(),
    {
        let ghost previous_seen = visited(keys.deep_view(), seen@, generation);
        let ghost previous_frontier = frontier.deep_view();
        let last = frontier.len() - 1;
        let current = frontier[last];
        frontier.truncate(last);
        proof {
            assert(previous_frontier.len() > 0);
            assert(current.deep_view() == previous_frontier.last());
            assert(current.deep_view() == previous_frontier[previous_frontier.len() as int - 1]);
            assert(frontier.deep_view() =~= previous_frontier.drop_last());
            assert(previous_frontier =~= frontier.deep_view().push(current.deep_view()));
            assert(reachable(kb@, start.deep_view(), current.deep_view()));
            assert forall|j: int| 0 <= j < frontier.len()
                implies reachable(kb@, start.deep_view(), frontier.deep_view()[j]) by {
                assert(frontier.deep_view()[j] == previous_frontier[j]);
            }
            assert(vertices(kb@).contains(current.deep_view()));
            assert(keys.deep_view().contains(current.deep_view()));
        }
        let Some(at) = index::find(keys, current) else {
            return false;
        };
        if seen[at] == generation {
            proof {
                assert(previous_seen.contains(current.deep_view()));
                processed_pop(kb@, start.deep_view(), previous_seen, frontier.deep_view(), current.deep_view());
            }
            continue;
        }
        proof { index::mark_visited(keys.deep_view(), seen@, at as int, generation); }
        seen.set(at, generation);
        let ghost remaining = frontier.deep_view();
        if !extend_frontier(kb, start, current, frontier) { return false; }
        proof {
            processed_insert(kb@, start.deep_view(), previous_seen, remaining, current.deep_view(), frontier.deep_view());
            assert(visited(keys.deep_view(), seen@, generation) == previous_seen.insert(current.deep_view()));
            assert(!previous_seen.contains(current.deep_view()));
            assert(vertices(kb@).difference(visited(keys.deep_view(), seen@, generation))
                == vertices(kb@).difference(previous_seen).remove(current.deep_view()));
        }
    }
    proof {
        assert(frontier.deep_view() =~= Seq::<Vertex>::empty());
        completed_search(kb@, start.deep_view(), visited(keys.deep_view(), seen@, generation));
    }
    true
}
pub(super) fn corrections_acyclic(kb: &KB, keys: &[Key<'_>]) -> (valid: bool)
    requires sorted(keys.deep_view()), keys.deep_view().no_duplicates(),
        keys.deep_view().to_set() == vertices(kb@),
        forall|i: int| 0 <= i < kb@.corrections.len() ==> correction_wf_spec(kb@.corrections[i], kb@),
    ensures valid == acyclic(kb@),
{
    let mut frontier = Vec::new();
    let mut seen = index::empty_stamps(keys.len());
    let claims = kb.claims();
    let mut i = 0;
    proof { assert(claims.deep_view().len() == claims.len()); }
    while i < claims.len()
        invariant i <= claims.len(), claims.deep_view() == kb@.claims, seen.len() == keys.len(),
            claims.len() == kb@.claims.len(),
            forall|j: int| 0 <= j < seen.len() ==> seen@[j] <= i,
            sorted(keys.deep_view()), keys.deep_view().no_duplicates(), keys.deep_view().to_set() == vertices(kb@),
            forall|j: int| 0 <= j < kb@.corrections.len() ==> correction_wf_spec(kb@.corrections[j], kb@),
            forall|j: int| 0 <= j < i ==> no_cycle_from(kb@, (kb@.claims[j].id, kb@.claims[j].revision)),
        decreases claims.len() - i,
    {
        let claim = &claims[i];
        proof {
            assert(claim@ == kb@.claims[i as int]);
            let key = (kb@.claims[i as int].id, kb@.claims[i as int].revision);
            assert(kb@.claims.map_values(|c: ClaimView| (c.id, c.revision))[i as int] == key);
            assert(vertices(kb@).contains(key));
        }
        if !search_start(kb, (claim.id().as_str(), claim.revision().as_str()), keys, i + 1, &mut frontier, &mut seen) { return false; }
        i += 1;
    }
    true
}
} // verus!
