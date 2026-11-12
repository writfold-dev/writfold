//! Write-verb latency (M3): each sample is one whole invocation, as the
//! CLI runs it: plan from the index, the atomic write, and the answer from
//! the refreshed index.
use super::{Result, summary};
use std::path::Path;
use std::time::Instant;
use writfold_kb::write::Write;
use writfold_kb::write::cite::CiteMake;
use writfold_kb::write::record::{Decision, Link, Ticket};
use writfold_kb::write::run::{Checkpoint, NewRun};
use writfold_kb::write::status::SetStatus;
use writfold_kb::{Request, Verb};

fn timed(
    request: &dyn Fn(Verb) -> Request,
    make: &dyn Fn(usize) -> Verb,
    runs: usize,
) -> Result<(f64, f64)> {
    let mut samples = Vec::with_capacity(runs);
    for n in 0..runs {
        let req = request(make(n));
        let started = Instant::now();
        writfold_kb::run(&req)?;
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(summary(samples))
}

pub fn writes(kb: &Path, request: &dyn Fn(Verb) -> Request, runs: usize) -> Result {
    println!(
        "{:<40} {:>10} {:>10}",
        "write (whole invocation)", "median ms", "p95 ms"
    );
    for (name, make) in cases(kb.to_string_lossy().into_owned()) {
        let (median, p95) = timed(request, make.as_ref(), runs)?;
        println!("{name:<40} {median:>10.1} {p95:>10.1}");
    }
    Ok(())
}

type Case = (&'static str, Box<dyn Fn(usize) -> Verb>);

fn cases(kb: String) -> Vec<Case> {
    let first: Vec<Case> = vec![
        (
            "checkpoint --new-run",
            Box::new(|n| {
                Verb::Write(Write::Checkpoint(Checkpoint {
                    run: None,
                    new_run: Some(NewRun {
                        project: "proj3".into(),
                        task: format!("Task {n}"),
                        mode: None,
                    }),
                    note: format!("Start {n}."),
                }))
            }),
        ),
        (
            "checkpoint (append)",
            Box::new(|n| {
                Verb::Write(Write::Checkpoint(Checkpoint {
                    run: None,
                    new_run: None,
                    note: format!("Entry {n}."),
                }))
            }),
        ),
        (
            "record ticket",
            Box::new(|n| {
                Verb::Write(Write::Ticket(Ticket {
                    ticket: format!("NEW-{n}"),
                    title: format!("New ticket {n}"),
                    assets: vec![format!("asset-{n}")],
                    ..Ticket::default()
                }))
            }),
        ),
    ];
    first.into_iter().chain(records(kb)).collect()
}

/// The record writes and the run reads.
fn records(kb: String) -> Vec<Case> {
    vec![
        (
            "record decision",
            Box::new(|n| {
                Verb::Write(Write::Decision(Decision {
                    title: format!("Decision {n} on sizing"),
                    target_source: "asset-1".into(),
                    ..Decision::default()
                }))
            }),
        ),
        (
            "record link",
            Box::new(|n| {
                Verb::Write(Write::Link(Link {
                    rel: "consumes".into(),
                    from: format!("asset-{n}"),
                    to: format!("asset-{}", n + 100),
                    ..Link::default()
                }))
            }),
        ),
        (
            "status --set (ticket)",
            Box::new(|n| {
                Verb::Write(Write::SetStatus(SetStatus {
                    target: format!("BEN-{}", 500 + n),
                    state: "blocked".into(),
                    ..SetStatus::default()
                }))
            }),
        ),
        (
            "cite make",
            Box::new(move |n| {
                Verb::Write(Write::CiteMake(CiteMake {
                    repo: kb.clone(),
                    path: format!("inventory/assets/asset-{}.json", 300 + n),
                    first: 1,
                    last: 1,
                    ..CiteMake::default()
                }))
            }),
        ),
        ("resume", Box::new(|_| Verb::Resume { run: None })),
        (
            "runs --open",
            Box::new(|_| Verb::Runs {
                open: true,
                project: None,
            }),
        ),
    ]
}
