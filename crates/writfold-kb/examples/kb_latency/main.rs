//! Warm-read and refresh latency over a generated KB of a few thousand
//! records.
//!
//! `cargo run --release -p writfold-kb --example kb_latency [-- RECORDS RUNS]`
//! generates an invented KB in a temporary repository, commits it, builds the
//! index once (cold), then times each verb `RUNS` times through the same
//! entry point the CLI uses (Git admission, one `status` spawn, the index
//! and the query), and finally times incremental refreshes of 50 changed
//! files, left dirty and committed, each followed by an `index verify`.
//! Prints median and p95 in milliseconds.
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Instant;
use writfold_kb::verbs::find::Filter;
use writfold_kb::verbs::impact::Direction;
use writfold_kb::{Request, Verb};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

mod generate;

use generate::{generate, git, write};

fn summary(mut samples: Vec<f64>) -> (f64, f64) {
    samples.sort_by(f64::total_cmp);
    let median = samples[samples.len() / 2];
    let p95 = samples[((samples.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len() - 1)];
    (median, p95)
}

fn time(request: &Request, runs: usize) -> Result<(f64, f64)> {
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        writfold_kb::run(request)?;
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(summary(samples))
}

fn main() -> Result {
    let mut args = std::env::args().skip(1);
    let records: usize = args.next().map_or(Ok(4000), |a| a.parse())?;
    let runs: usize = args.next().map_or(Ok(21), |a| a.parse())?;
    let base = std::env::temp_dir().join(format!("writfold-kb-latency-{}", std::process::id()));
    let kb = base.join("kb");
    generate(&kb, records)?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-qm", "generated"])?;
    let request = |verb: Verb| Request {
        verb,
        kb: Some(kb.clone()),
        cwd: PathBuf::from("/"),
        budget: 1000,
        cursor: None,
        json: false,
        now: None,
        probe: writfold_kb::write::no_probe,
    };
    let started = Instant::now();
    let status = writfold_kb::run(&request(Verb::IndexRebuild))?;
    println!(
        "cold rebuild: {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    print!(
        "{}",
        status
            .text
            .lines()
            .filter(|l| l.contains("records:") || l.contains("count:") || l.starts_with('#'))
            .map(|l| format!("  {l}\n"))
            .collect::<String>()
    );
    let result = report(&request, runs).and_then(|()| refreshes(&kb, &request, runs));
    std::fs::remove_dir_all(&base)?;
    result
}

fn find(text: &str, signatures: bool) -> Verb {
    Verb::Find {
        text: text.into(),
        filter: Filter::default(),
        signatures,
    }
}

fn verbs() -> Vec<(&'static str, Verb)> {
    let brief = |project: Option<&str>| Verb::Brief {
        project: project.map(str::to_owned),
        all: project.is_none(),
    };
    let context = |t: &str| Verb::Context {
        target: t.into(),
        depth: 2,
    };
    let why = |t: &str| Verb::Why {
        text: t.into(),
        sections: Vec::new(),
    };
    let impact = |t: &str, inferred| Verb::Impact {
        target: t.into(),
        depth: 2,
        direction: Direction::Both,
        inferred,
    };
    vec![
        ("brief proj3", brief(Some("proj3"))),
        ("brief --all", brief(None)),
        ("context asset-7", context("asset-7")),
        ("context ben 12", context("ben 12")),
        ("why queue sizing", why("queue sizing")),
        (
            "why decision/2026-01-01-choice-2",
            why("decision/2026-01-01-choice-2"),
        ),
        (
            "stale --offline",
            Verb::Stale {
                scope: None,
                offline: true,
            },
        ),
        (
            "stale proj3",
            Verb::Stale {
                scope: Some("proj3".into()),
                offline: false,
            },
        ),
        ("find BEN-12 (exact)", find("BEN-12", false)),
        ("find queue sizing (text)", find("queue sizing", false)),
        (
            "find atency of serv (substring)",
            find("atency of serv", false),
        ),
        ("find asset-71x (typo)", find("asset-71x", false)),
        ("match fsync took", find("fsync took", true)),
        (
            "status BEN-12",
            Verb::Status {
                target: "BEN-12".into(),
            },
        ),
        (
            "status proj3",
            Verb::Status {
                target: "proj3".into(),
            },
        ),
        ("impact asset-7", impact("asset-7", false)),
        ("impact asset-7 --include inferred", impact("asset-7", true)),
        (
            "uses asset-7",
            Verb::Uses {
                target: "asset-7".into(),
            },
        ),
        (
            "timeline asset-7",
            Verb::Timeline {
                target: "asset-7".into(),
            },
        ),
        ("index status", Verb::IndexStatus),
    ]
}

fn report(request: &dyn Fn(Verb) -> Request, runs: usize) -> Result {
    println!("{:<40} {:>10} {:>10}", "verb (warm)", "median ms", "p95 ms");
    for (name, verb) in verbs() {
        let (median, p95) = time(&request(verb), runs)?;
        println!("{name:<40} {median:>10.1} {p95:>10.1}");
    }
    Ok(())
}

/// Append a line to 50 ticket records (a different 50 each round).
fn touch_fifty(kb: &Path, round: usize) -> Result {
    for k in 0..50 {
        let path = format!("ktlo/2026/ben-{}-ticket.md", (round * 50 + k) % 1000);
        let mut text = std::fs::read_to_string(kb.join(&path))?;
        text.push_str(&format!("\nRound {round}: see asset-{k}.\n"));
        write(kb, &path, &text)?;
    }
    Ok(())
}

/// Time the read that refreshes 50 changed files (the `index status` verb,
/// so the query itself is small), dirty and then committed; check the
/// refreshed index equals a rebuild after each kind.
fn refreshes(kb: &Path, request: &dyn Fn(Verb) -> Request, runs: usize) -> Result {
    println!(
        "{:<40} {:>10} {:>10}",
        "refresh (50 files)", "median ms", "p95 ms"
    );
    for committed in [false, true] {
        let mut samples = Vec::with_capacity(runs);
        let mut labels = Vec::new();
        for round in 0..runs {
            // Start each round clean and indexed, so exactly 50 files change.
            git(kb, &["checkout", "-q", "--", "."])?;
            writfold_kb::run(&request(Verb::IndexStatus))?;
            touch_fifty(kb, round + usize::from(committed) * runs)?;
            if committed {
                git(kb, &["commit", "-qam", &format!("round {round}")])?;
            }
            let started = Instant::now();
            let output = writfold_kb::run(&request(Verb::IndexStatus))?;
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
            labels.extend(output.text.lines().last().and_then(|t| {
                t.split(' ')
                    .find(|p| p.starts_with("index="))
                    .map(str::to_owned)
            }));
        }
        let (median, p95) = summary(samples);
        let name = if committed {
            "committed (diff-tree)"
        } else {
            "dirty (status)"
        };
        println!("{name:<40} {median:>10.1} {p95:>10.1}");
        labels.sort();
        labels.dedup();
        let started = Instant::now();
        let verify = writfold_kb::run(&request(Verb::IndexVerify))?;
        println!(
            "  states {labels:?}; index verify {:.1} ms, exit {} (0 = equals a rebuild)",
            started.elapsed().as_secs_f64() * 1000.0,
            verify.exit
        );
    }
    Ok(())
}
