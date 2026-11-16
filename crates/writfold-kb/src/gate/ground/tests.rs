//! A raw retrieval pointer cannot inherit a real source/gate's checked verdict.
#[path = "../../../tests/checklist/mod.rs"]
mod checklist;

use checklist::{RUN, STALL, define, run_unit};
use crate::gate::ground::{self, Context, EvidenceInput};
use crate::gate::status::{self, Filings};
use crate::gate::subject::Subject;
use crate::session::Session;
use crate::support::{NOW, TestResult, world};
use crate::workspace::{GitTool, discover};
use writfold_core::{assess, checklist as kernel};

#[test]
fn raw_pointer_cannot_support_checklist_gate_or_citation_evidence() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    run_unit(&w)?;
    let ws = discover(Some(&w.kb), &w.base)?;
    let tool = GitTool::locate()?;
    let session = Session::open(&ws, Some(&tool), false, NOW)?;
    let subject = Subject {
        top: w.code.clone(),
        git_dir: w.code.join(".git"),
        name: "demo-service".into(),
    };
    let run_dir = format!("runs/{RUN}");
    let filings = Filings::read(&w.kb, &run_dir, RUN);
    let (plain, _) = status::judge_plain(&session, &subject, &filings)?;
    let cx = Context {
        session: &session,
        subject: &subject,
        plain: &plain,
        filings: &filings,
        run_dir: &run_dir,
    };
    for reference in [STALL, "gate:unit"] {
        let checked = ground::resolve(&cx, EvidenceInput::CheckedReference(reference))?;
        assert_eq!(checked.grounding, assess::Grounding::Grounded);
        for input in [
            EvidenceInput::SemanticPointer(reference),
            EvidenceInput::LexicalPointer(reference),
        ] {
            let pointer = ground::resolve(&cx, input)?;
            assert_eq!(pointer.grounding, assess::Grounding::Ungrounded);
            assert!(!pointer.recorded);
            assert_eq!(pointer.path, None);
            let item = kernel::item(Some(kernel::Row {
                outcome: kernel::Outcome::Supported,
                current: true,
                evidence: 1,
                grounded: pointer.grounding != assess::Grounding::Ungrounded,
            }));
            assert_eq!(item, kernel::Item::Stale);
            let basis = assess::basis(&[pointer.grounding]);
            assert!(!assess::met(assess::Verdict::Entailed, basis));
        }
    }
    let moved = STALL.replace("state=done", "state=open");
    let checked = ground::resolve(&cx, EvidenceInput::CheckedReference(&moved))?;
    assert_eq!(checked.grounding, assess::Grounding::Ungrounded);
    Ok(())
}
