use super::{RefFact, Refusal, closure, covers};
use alloc::vec;
use alloc::vec::Vec;

fn ids(names: &[&str]) -> Vec<Vec<u8>> {
    names.iter().map(|name| name.as_bytes().to_vec()).collect()
}

#[test]
fn covers_finds_the_first_tool_not_granted() {
    let granted = ids(&["git.diff", "kb.find"]);
    assert_eq!(covers(&ids(&[]), &granted), None);
    assert_eq!(covers(&ids(&["kb.find", "git.diff"]), &granted), None);
    assert_eq!(
        covers(&ids(&["kb.find", "shell", "web"]), &granted),
        Some(1)
    );
    // the last required tool counts too
    assert_eq!(covers(&ids(&["kb.find", "web"]), &granted), Some(1));
    assert_eq!(covers(&ids(&["kb.find"]), &ids(&[])), Some(0));
}

#[test]
fn covers_compares_whole_ids() {
    let granted = ids(&["kb.find"]);
    assert_eq!(covers(&ids(&["kb"]), &granted), Some(0));
    assert_eq!(covers(&ids(&["kb.finder"]), &granted), Some(0));
}

#[test]
fn closure_refuses_the_first_unresolved_reference_first() {
    let granted = ids(&["kb.find"]);
    let missing_tool = ids(&["shell"]);
    let refs = [RefFact::Found, RefFact::WrongKind, RefFact::Missing];
    assert_eq!(
        closure(&refs, &missing_tool, &granted),
        Err(Refusal::Reference(1))
    );
    for bad in [RefFact::Missing, RefFact::WrongKind, RefFact::Twice] {
        assert_eq!(
            closure(&[RefFact::Found, RefFact::Found, bad], &[], &granted),
            Err(Refusal::Reference(2))
        );
    }
}

#[test]
fn closure_then_checks_tools() {
    let granted = ids(&["kb.find"]);
    let refs = [RefFact::Found, RefFact::Found];
    assert_eq!(
        closure(&refs, &ids(&["kb.find", "shell"]), &granted),
        Err(Refusal::Tool(1))
    );
    assert_eq!(closure(&refs, &ids(&["kb.find"]), &granted), Ok(()));
    assert_eq!(closure(&[], &[], &[]), Ok(()));
}

#[test]
fn more_skills_never_admit() {
    let granted = ids(&["kb.find"]);
    let cases: Vec<(Vec<RefFact>, Vec<Vec<u8>>)> = vec![
        (vec![RefFact::Missing], ids(&[])),
        (vec![RefFact::Found], ids(&["shell"])),
    ];
    for (refs, required) in cases {
        assert!(closure(&refs, &required, &granted).is_err());
        let mut more_refs = refs.clone();
        more_refs.push(RefFact::Found);
        let mut more_required = required.clone();
        more_required.extend(ids(&["kb.find"]));
        assert!(closure(&more_refs, &more_required, &granted).is_err());
    }
}
