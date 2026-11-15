//! `record investigation` and `propose investigation`: a new, open
//! investigation in the layout the KB already reads, with the sections
//! `status --set concluded` checks; refused on a taken ID or an unknown
//! asset, a usage error for a bad ticket, signature or title.
mod support;

use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::proposal::{self, ProposalVerb, Propose};
use writfold_kb::write::Write;
use writfold_kb::write::investigation::Investigation;
use writfold_kb::write::status::SetStatus;

const ID: &str = "2026/2026-11-11-flush-stalls-on-replicas";
const DIR: &str = "investigations/2026/2026-11-11-flush-stalls-on-replicas";

fn call() -> Investigation {
    Investigation {
        title: "Flush stalls on replicas".into(),
        tickets: vec!["hbr-99".into()],
        assets: vec!["harbor-db".into()],
        alerts: vec!["harbor-5xx".into()],
        signatures: vec!["fsync-stall".into()],
        question: Some("Why do replicas stall on flush?".into()),
        ..Investigation::default()
    }
}

fn record(i: Investigation) -> Verb {
    Verb::Write(Write::Investigation(i))
}

fn exit_of(w: &World, verb: Verb) -> u8 {
    match writfold_kb::run(&w.request(verb)) {
        Ok(output) => output.exit,
        Err(error) => error.exit_code(),
    }
}

fn readme(w: &World) -> TestResult<String> {
    Ok(std::fs::read_to_string(w.kb.join(DIR).join("README.md"))?)
}

#[test]
fn record_writes_an_open_investigation_with_its_edges() -> TestResult {
    let w = world(&[])?;
    let (text, exit) = w.run(&w.request(record(call())))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.starts_with(&format!(
            "investigation inv:{ID} [open] Flush stalls on replicas"
        )),
        "{text}"
    );
    assert!(w.kb.join(DIR).join("evidence").is_dir());
    let body = readme(&w)?;
    let head = "---\nid: 2026/2026-11-11-flush-stalls-on-replicas\ntitle: Flush stalls on replicas\ntrigger: manual\ntickets: [HBR-99]\nalerts: [harbor-5xx]\nassets: [harbor-db]\nstatus: open\nsignatures: [fsync-stall]\ncreated: 2026-11-11\nupdated: 2026-11-11\nsummary: \n---\n";
    assert!(body.starts_with(head), "{body}");
    assert!(
        body.contains("## Question\n\nWhy do replicas stall on flush?\n"),
        "{body}"
    );
    let (text, _) = w.run(&w.request(Verb::Context {
        target: format!("inv:{ID}"),
        depth: 1,
    }))?;
    for edge in ["harbor-db", "sig:fsync-stall"] {
        assert!(text.contains(edge), "{edge}: {text}");
    }
    let (text, _) = w.run(&w.request(Verb::Uses {
        target: "HBR-99".into(),
    }))?;
    assert!(text.contains(&format!("inv:{ID}")), "{text}");
    let (text, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !text.contains("[differs]"), "{text}");
    // The same title the same day is the same ID: refused, nothing written.
    let before = readme(&w)?;
    assert_eq!(exit_of(&w, record(call())), 3);
    assert_eq!(readme(&w)?, before);
    Ok(())
}

#[test]
fn the_template_concludes_once_its_sections_are_filled() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(record(call())))?;
    let conclude = || record_status(&w);
    assert_eq!(conclude(), 3, "an empty write-up is not concluded");
    let filled = readme(&w)?
        .replace(
            "## Hypotheses\n",
            "## Hypotheses\n\n- H1: supported, the replica volume is slow\n",
        )
        .replace(
            "## Checked against\n",
            "## Checked against\n\nfsync traces\n",
        )
        .replace(
            "## Scope and limits\n",
            "## Scope and limits\n\nTwo replicas only.\n",
        );
    std::fs::write(w.kb.join(DIR).join("README.md"), &filled)?;
    assert_eq!(conclude(), 0);
    assert!(readme(&w)?.contains("status: concluded"));
    Ok(())
}

fn record_status(w: &World) -> u8 {
    exit_of(
        w,
        Verb::Write(Write::SetStatus(SetStatus {
            target: format!("inv:{ID}"),
            state: "concluded".into(),
            ..SetStatus::default()
        })),
    )
}

#[test]
fn bad_arguments_are_refused_before_anything_is_written() -> TestResult {
    let w = world(&[])?;
    let usage = [
        Investigation {
            tickets: vec!["not a ticket".into()],
            ..call()
        },
        Investigation {
            signatures: vec!["Bad Slug".into()],
            ..call()
        },
        Investigation {
            title: "two\nlines".into(),
            ..call()
        },
        Investigation {
            trigger: Some("a\nb".into()),
            ..call()
        },
    ];
    for i in usage {
        assert_eq!(exit_of(&w, record(i)), 1);
    }
    let unknown = Investigation {
        assets: vec!["no-such-asset".into()],
        ..call()
    };
    assert_eq!(exit_of(&w, record(unknown)), 3);
    // Tickets and signatures need not exist yet.
    let early = Investigation {
        tickets: vec!["NEW-1".into()],
        signatures: vec!["later".into()],
        ..call()
    };
    assert_eq!(exit_of(&w, record(early)), 0);
    assert_eq!(
        support::git(&w.kb, &["status", "--porcelain"])?
            .lines()
            .count(),
        1
    );
    Ok(())
}

#[test]
fn propose_and_accept_write_the_bytes_record_writes() -> TestResult {
    let direct = world(&[])?;
    direct.run(&direct.request(record(call())))?;
    let w = world(&[])?;
    let propose = Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: Write::Investigation(call()),
        session: Some("s1".into()),
        run: None,
        store: None,
    })));
    let (text, exit) = w.run(&w.request(propose))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("investigation: Flush stalls on replicas"),
        "{text}"
    );
    let id = proposal::all(&w.kb.join(".git/writfold"))
        .into_iter()
        .map(|p| p.id)
        .next()
        .ok_or("a proposal")?;
    let (text, _) = w.run(&w.request(Verb::Proposal(ProposalVerb::Show { id: id.clone() })))?;
    assert!(
        text.contains("call: writfold kb record investigation --alerts harbor-5xx")
            && text.contains("--title 'Flush stalls on replicas'"),
        "{text}"
    );
    let (text, exit) = w.run(&w.request(Verb::Proposal(ProposalVerb::Accept { id })))?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(readme(&w)?, readme(&direct)?);
    Ok(())
}
