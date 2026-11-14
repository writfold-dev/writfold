mod support;

use support::plan::world;
use support::{RUN, TestResult};
use writfold_kb::Verb;
use writfold_kb::proposal::{ProposalVerb, Propose, Store};
use writfold_kb::write::{Write, record::Ticket};
use writfold_tui::app::{App, View};
use writfold_tui::{frame, side};

#[test]
fn tracked_proposals_are_not_labelled_as_an_only_local_queue() -> TestResult {
    let w = world("tracked-inbox")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Inbox);
    w.verb(
        Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
            write: Write::Ticket(Ticket {
                ticket: "HBR-211".into(),
                title: "Review a tracked proposal".into(),
                ..Ticket::default()
            }),
            session: Some("tracked-view".into()),
            run: Some(RUN.into()),
            store: Some(Store::Tracked),
        }))),
        false,
    )?;
    assert!(
        app.poll(),
        "a tracked proposal must appear on the next tick"
    );
    let content = frame::detail(&app, View::Inbox, None, true, true);
    assert!(!content.lines[0].text.contains("local; not in the KB"));
    assert!(content.lines.iter().any(|l| l.text.contains("!tracked")));
    assert!(
        content
            .lines
            .iter()
            .any(|l| l.text.contains("accept: writfold kb proposals accept"))
    );
    let compact = side::compact(&app, View::Inbox);
    assert!(!compact[0].text.contains("(local; not in the KB)"));
    Ok(())
}
