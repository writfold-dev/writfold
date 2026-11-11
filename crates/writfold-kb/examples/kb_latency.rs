//! Warm-read latency over a generated KB of a few thousand records.
//!
//! `cargo run --release -p writfold-kb --example kb_latency [-- RECORDS RUNS]`
//! generates an invented KB in a temporary repository, commits it, builds the
//! index once (cold), then times each verb `RUNS` times through the same
//! entry point the CLI uses (Git admission, one `status` spawn, the index
//! and the query). Prints median and p95 per verb in milliseconds.
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use writfold_kb::{Request, Verb};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn write(root: &Path, path: &str, text: &str) -> Result {
    let full = root.join(path);
    std::fs::create_dir_all(full.parent().ok_or("parent")?)?;
    std::fs::write(full, text)?;
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let status = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=bench",
            "-c",
            "user.email=bench@example.invalid",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn gen_projects(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for p in 0..20 {
        write(
            root,
            &format!("proj{p}/STATUS.md"),
            &format!(
                "---\nphase: build\nowner: team{p}\n---\n\n# Project {p}\n\nChecked ⟦cite kind=web url=https://example.invalid/{p} seen=2026-10-01T00:00Z ttl=30d⟧.\n"
            ),
        )?;
        write(
            root,
            &format!("proj{p}/README.md"),
            &format!("# Project {p}\n"),
        )?;
    }
    Ok(())
}

fn gen_assets(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for a in 0..2 * unit {
        let body = format!(
            "{{\"id\": \"asset-{a}\", \"kind\": \"service\", \"name\": \"Service {a}\", \"aliases\": [\"svc{a}\"], \"lifecycle\": \"active\", \"projects\": [\"proj{}\"], \"verified_at\": \"2026-10-01\", \"ttl\": \"90d\"}}\n",
            a % 20
        );
        write(root, &format!("inventory/assets/asset-{a}.json"), &body)?;
        let rel = format!(
            "{{\"type\": \"depends-on\", \"from\": \"asset-{a}\", \"to\": \"asset-{}\", \"status\": \"explicit\"}}\n",
            (a + 1) % (2 * unit)
        );
        write(
            root,
            &format!(
                "inventory/relationships/depends-on__asset-{a}__asset-{}.json",
                (a + 1) % (2 * unit)
            ),
            &rel,
        )?;
    }
    Ok(())
}

fn gen_tickets(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for t in 0..3 * unit {
        let status = if t % 3 == 0 { "done" } else { "in-progress" };
        let text = format!(
            "---\nticket: BEN-{t}\ntitle: Ticket {t} about latency of service {}\nassets: [asset-{}]\nstatus: {status}\nprs: []\nsummary: Work item {t}.\ncreated: 2026-10-{:02}\nupdated: 2026-10-{:02}\npromoted_to: proj{}\n---\n\n# BEN-{t}\n\n## Symptom\n\nSlow path {t}.\n\n## Outcome\n\nResolved by batching {t}.\n",
            t % (2 * unit),
            t % (2 * unit),
            1 + t % 28,
            1 + t % 28,
            t % 20
        );
        write(root, &format!("ktlo/2026/ben-{t}-ticket.md"), &text)?;
    }
    Ok(())
}

fn gen_decisions(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for d in 0..2 * unit {
        let status = ["accepted", "draft", "superseded", "applied"][d % 4];
        let text = format!(
            "---\nid: decision/2026-01-01-choice-{d}\nkind: decision\ntitle: Choice {d} for queue sizing\nstatus: {status}\ncreated: 2026-01-01\nupdated: 2026-02-{:02}\nproject: proj{}\nreferences: [asset-{d}]\ndecided_by: review\nsuperseded_by: {}\n---\n\n## Decision\n\nUse queue size {d} because the flush budget allows it.\n\n## Context\n\nBackground {d}.\n",
            1 + d % 28,
            d % 20,
            if status == "superseded" {
                format!("decision/2026-01-01-choice-{}", d + 1)
            } else {
                String::new()
            }
        );
        write(
            root,
            &format!("proposals/decision/2026-01-01-choice-{d}/proposal.md"),
            &text,
        )?;
    }
    Ok(())
}

fn gen_investigations(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for i in 0..unit {
        let text = format!(
            "---\nid: 2026/2026-03-01-inv-{i}\ntitle: Investigation {i}\nstatus: {}\ntickets: [BEN-{i}]\nassets: [asset-{i}]\nsignatures: [sig-{}]\ncreated: 2026-03-01\n---\n\n## Conclusion\n\nFound cause {i}.\n",
            if i % 2 == 0 { "concluded" } else { "open" },
            i % 50
        );
        write(
            root,
            &format!("investigations/2026/2026-03-01-inv-{i}/README.md"),
            &text,
        )?;
    }
    Ok(())
}

fn gen_signatures(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for s in 0..50 {
        write(
            root,
            &format!("signatures/sig-{s}.md"),
            &format!("---\ntitle: Signature {s}\nstatus: confirmed\nassets: [asset-{s}]\n---\n"),
        )?;
    }
    Ok(())
}

fn gen_runs(root: &Path, unit: usize) -> Result {
    let _ = unit;
    for r in 0..20 {
        write(
            root,
            &format!(
                "runs/run-2026110{}T000000Z-00000{r}/manifest.json",
                r % 9 + 1
            ),
            &format!(
                "{{\"run_id\": \"run-{r}\", \"task\": \"Task {r}\", \"project\": \"proj{r}\", \"created\": \"2026-11-0{}T00:00:00Z\", \"state\": \"open\"}}\n",
                r % 9 + 1
            ),
        )?;
        write(
            root,
            &format!(
                "runs/run-2026110{}T000000Z-00000{r}/checkpoint.md",
                r % 9 + 1
            ),
            &format!(
                "\n## 2026-11-0{}T00:00:00Z\n\nProgress note {r}.\n",
                r % 9 + 1
            ),
        )?;
    }
    Ok(())
}

/// About `records` records across every kind, in 20 projects.
fn generate(root: &Path, records: usize) -> Result {
    let unit = records / 10;
    gen_projects(root, unit)?;
    gen_assets(root, unit)?;
    gen_tickets(root, unit)?;
    gen_decisions(root, unit)?;
    gen_investigations(root, unit)?;
    gen_signatures(root, unit)?;
    gen_runs(root, unit)?;
    Ok(())
}

fn time(request: &Request, runs: usize) -> Result<(f64, f64)> {
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        writfold_kb::run(request)?;
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    let median = samples[samples.len() / 2];
    let p95 = samples[((samples.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len() - 1)];
    Ok((median, p95))
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
            .filter(|l| l.contains("records:") || l.starts_with('#'))
            .map(|l| format!("  {l}\n"))
            .collect::<String>()
    );
    let result = report(&request, runs);
    std::fs::remove_dir_all(&base)?;
    result
}

fn verbs() -> Vec<(&'static str, Verb)> {
    vec![
        (
            "brief proj3",
            Verb::Brief {
                project: Some("proj3".into()),
                all: false,
            },
        ),
        (
            "brief --all",
            Verb::Brief {
                project: None,
                all: true,
            },
        ),
        (
            "context asset-7",
            Verb::Context {
                target: "asset-7".into(),
                depth: 2,
            },
        ),
        (
            "context ben 12",
            Verb::Context {
                target: "ben 12".into(),
                depth: 2,
            },
        ),
        (
            "why queue sizing",
            Verb::Why {
                text: "queue sizing".into(),
                sections: Vec::new(),
            },
        ),
        (
            "why decision/2026-01-01-choice-2",
            Verb::Why {
                text: "decision/2026-01-01-choice-2".into(),
                sections: Vec::new(),
            },
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
        ("index status", Verb::IndexStatus),
    ]
}

fn report(request: &dyn Fn(Verb) -> Request, runs: usize) -> Result {
    println!("{:<36} {:>10} {:>10}", "verb (warm)", "median ms", "p95 ms");
    for (name, verb) in verbs() {
        let (median, p95) = time(&request(verb), runs)?;
        println!("{name:<36} {median:>10.1} {p95:>10.1}");
    }
    Ok(())
}
