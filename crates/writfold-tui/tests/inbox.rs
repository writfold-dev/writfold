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

#[test]
fn inbox_view_leads_with_the_breakdown_its_tab_badge_sums() -> TestResult {
    let w = world("inbox-breakdown")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Inbox);
    let buffer = support::projection::render(&mut app, 100, 30)?;
    let screen = support::projection::rows(&buffer);
    assert!(screen[1].contains("[4 inbox] 3"), "{}", screen[1]);
    let main = writfold_tui::layout::areas(100, 30).main;
    let first: String = screen[usize::from(main.y) + 1]
        .chars()
        .skip(usize::from(main.x))
        .collect();
    assert!(first.contains("drafts=1 proposals=2"), "{first}");
    Ok(())
}
