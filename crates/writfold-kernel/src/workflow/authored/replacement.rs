use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub open spec fn replacement_ids_unique(rows: Seq<AuthoredReplacementView>) -> bool {
    forall|i: int, j: int| 0 <= i < j < rows.len() ==> rows[i].id != rows[j].id
}

// A finite resolving chain must end in a row with no successor, must not revisit
// the original proposal, and must resolve every hop. With unique row ids, a
// terminal chain of at most rows.len() hops cannot contain a cycle.
pub open spec fn replacement_resolves(rows: Seq<AuthoredReplacementView>, origin: Seq<char>,
    target: Seq<char>, remaining: nat) -> bool
    decreases remaining,
{
    remaining > 0 && target != origin && exists|i: int| 0 <= i < rows.len()
        && rows[i].id == target && match rows[i].superseded_by {
            None => true,
            Some(next) => replacement_resolves(rows, origin, next, (remaining - 1) as nat),
        }
}

pub open spec fn authored_supersession_valid_spec(p: AuthoredProposalView) -> bool {
    replacement_ids_unique(p.proposals)
        && (exists|i: int| 0 <= i < p.proposals.len() && p.proposals[i].id == p.id)
        && match p.superseded_by {
            None => false,
            Some(target) => replacement_resolves(p.proposals, p.id, target, p.proposals.len()),
        }
}

fn unique_rows(rows: &[AuthoredReplacement]) -> (unique: bool)
    ensures unique == replacement_ids_unique(rows.deep_view()),
{
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(),
            forall|k: int, j: int| 0 <= k < j < i ==>
                rows.deep_view()[k].id != rows.deep_view()[j].id,
        decreases rows.len() - i,
    {
        let mut j = 0;
        while j < i
            invariant j <= i < rows.len(),
                forall|k: int, j: int| 0 <= k < j < i ==>
                    rows.deep_view()[k].id != rows.deep_view()[j].id,
                forall|k: int| 0 <= k < j ==> rows.deep_view()[k].id != rows.deep_view()[i as int].id,
            decreases i - j,
        {
            if rows[j].id() == rows[i].id() {
                proof {
                    assert(rows.deep_view()[j as int].id == rows.deep_view()[i as int].id);
                    if replacement_ids_unique(rows.deep_view()) {
                        assert(rows.deep_view()[j as int].id != rows.deep_view()[i as int].id);
                    }
                }
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

fn find_row<'a>(rows: &'a [AuthoredReplacement], target: &ProposalId)
    -> (found: Option<&'a AuthoredReplacement>)
    ensures match found {
        Some(row) => rows.deep_view().contains(row@) && row@.id == target@,
        None => forall|i: int| 0 <= i < rows.len() ==> rows.deep_view()[i].id != target@,
    },
{
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(),
            forall|j: int| 0 <= j < i ==> rows.deep_view()[j].id != target@,
        decreases rows.len() - i,
    {
        if rows[i].id() == target {
            let row = &rows[i];
            proof {
                assert(row@ == rows.deep_view()[i as int]);
                assert(rows.deep_view().contains(row@));
            }
            return Some(row);
        }
        i += 1;
    }
    None
}

proof fn resolve_step(rows: Seq<AuthoredReplacementView>, origin: Seq<char>,
    row: AuthoredReplacementView, remaining: nat)
    requires replacement_ids_unique(rows), rows.contains(row), remaining > 0,
    ensures replacement_resolves(rows, origin, row.id, remaining) ==
        (row.id != origin && match row.superseded_by {
            None => true,
            Some(next) => replacement_resolves(rows, origin, next, (remaining - 1) as nat),
        }),
{
    let index = choose|i: int| 0 <= i < rows.len() && rows[i] == row;
    assert forall|i: int| 0 <= i < rows.len() && rows[i].id == row.id
        implies rows[i] == row by {
        if i < index { assert(rows[i].id != rows[index].id); }
        if index < i { assert(rows[index].id != rows[i].id); }
    }
}

pub fn authored_supersession_valid(proposal: &AuthoredProposal) -> (valid: bool)
    ensures valid == authored_supersession_valid_spec(proposal@),
{
    let rows = proposal.proposals();
    if !unique_rows(rows) || find_row(rows, proposal.id()).is_none() { return false; }
    let Some(mut target) = proposal.superseded_by().as_ref() else { return false; };
    let mut remaining = rows.len();
    while remaining > 0
        invariant remaining <= rows.len(), replacement_ids_unique(rows.deep_view()),
            authored_supersession_valid_spec(proposal@) ==
                replacement_resolves(rows.deep_view(), proposal@.id, target@, remaining as nat),
        decreases remaining,
    {
        if target == proposal.id() { return false; }
        let Some(row) = find_row(rows, target) else { return false; };
        proof { resolve_step(rows.deep_view(), proposal@.id, row@, remaining as nat); }
        match row.superseded_by() {
            Some(next) => { target = next; remaining -= 1; },
            None => return true,
        }
    }
    false
}

} // verus!
