//! `gate run <id>` on an assessed gate: read the Weft source, run the
//! pinned binary on it, resolve the citations above each assumption the
//! report names, judge each check, and record one receipt in the run
//! through the write protocol. Weft's report stays local and untracked,
//! under `<KB git dir>/writfold/gates/<run>/<receipt>/weft-report.json`;
//! the receipt holds its digest.
use super::receipt::{self, Assessed, Assumed, Cited, Judged as Check};
use super::report::{self, Report};
use super::spec::Spec;
use super::weft;
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::gate::def::Definition;
use crate::gate::ground::{self, Context};
use crate::gate::status::{self, Filings};
use crate::gate::subject::Subject;
use crate::index::query::Rec;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic};
use crate::{runs, time};
use sha2::{Digest, Sha256};
use writfold_core::assess::{self, Grounding, Verdict};

/// What the run decided, before it is written.
struct Judgement {
    assumptions: Vec<Assumed>,
    checks: Vec<Check>,
    missing: Vec<String>,
}

/// Ground every assumption and judge every check of a complete report.
fn judgement(
    cx: &Context<'_, '_>,
    report: &Report,
    required: &[String],
) -> Result<std::result::Result<Judgement, String>> {
    let mut assumptions = Vec::new();
    for a in &report.assumptions {
        let mut citations = Vec::new();
        for (binding, reference) in &a.citations {
            let r = ground::resolve(cx, reference)?;
            citations.push(Cited {
                binding: binding.clone(),
                reference: reference.clone(),
                grounding: r.grounding,
                standing: r.standing,
                path: r.path,
            });
        }
        let groundings: Vec<Grounding> = citations.iter().map(|c| c.grounding).collect();
        assumptions.push(Assumed {
            name: a.name.clone(),
            grounding: assess::assumption(&groundings),
            citations,
        });
    }
    let mut checks = Vec::new();
    for c in &report.checks {
        let Some(verdict) = Verdict::parse(&c.result) else {
            return Ok(Err(format!(
                "Weft result {:?} for {} is not one Writfold reads",
                c.result, c.name
            )));
        };
        let mut premises = Vec::new();
        for p in &c.premises {
            match assumptions.iter().find(|a| &a.name == p) {
                Some(a) => premises.push(a.grounding),
                None => {
                    return Ok(Err(format!(
                        "check {} rests on {p}, which is not an assumption",
                        c.name
                    )));
                }
            }
        }
        let basis = assess::basis(&premises);
        checks.push(Check {
            name: c.name.clone(),
            required: required.contains(&c.name),
            verdict,
            weft_basis: c.weft_basis.clone(),
            basis,
            premises: c.premises.clone(),
            met: assess::met(verdict, basis),
        });
    }
    let missing = required
        .iter()
        .filter(|r| !report.checks.iter().any(|c| &c.name == *r))
        .cloned()
        .collect();
    Ok(Ok(Judgement {
        assumptions,
        checks,
        missing,
    }))
}

/// The subject's commit, why there is no assessment (if so), and the
/// judgement of a report: citations resolved against the run's other
/// gates and the KB as they stand now.
fn decide(
    ws: &Workspace,
    tool: &GitTool,
    (rec, subject): (&Rec, &Subject),
    spec: &Spec,
    ran: &weft::Ran,
    source_hex: &str,
    now: i64,
) -> Result<(String, Option<String>, Judgement)> {
    let run_dir = runs::dir(rec);
    let session = Session::open(ws, Some(tool), false, now)?;
    let filings = Filings::read(&ws.root, &run_dir, &rec.id);
    let mut plain = status::judge(&session, subject, &filings)?;
    plain.gates.retain(|j| j.assessment.is_none());
    let commit = plain.snapshot.commit.as_str().to_owned();
    let cx = Context {
        session: &session,
        subject,
        plain: &plain,
        filings: &filings,
        run_dir: &run_dir,
    };
    let empty = Judgement {
        assumptions: Vec::new(),
        checks: Vec::new(),
        missing: Vec::new(),
    };
    let Some(bytes) = &ran.report else {
        return Ok((commit, ran.error.clone(), empty));
    };
    Ok(match report::parse(bytes, source_hex) {
        Err(why) => (commit, Some(why), empty),
        Ok(r) => match judgement(&cx, &r, &spec.checks)? {
            Ok(j) => (commit, None, j),
            Err(why) => (commit, Some(why), empty),
        },
    })
}

/// Run an assessed gate for `rec` on `subject`.
pub fn run(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    rec: &Rec,
    subject: &Subject,
    d: &Definition,
    now: i64,
) -> Result<Output> {
    let spec = d
        .assessed
        .as_ref()
        .ok_or_else(|| KbError::Refused("not an assessed gate".into()))?;
    let run_dir = runs::dir(rec);
    let file = spec
        .source
        .file(&d.dir, &ws.root.join(&run_dir))
        .map_err(|why| KbError::Refused(format!("gate {}: Weft source: {why}", d.id)))?;
    let bytes = std::fs::read(&file)?;
    let source_hex = crate::index::hex(&Sha256::digest(&bytes));
    let ran = weft::run(&spec.pin, &file, d.timeout_seconds)?;
    let stem = crate::gate::run::name(&d.id, now);
    let local = crate::gate::run::local_dir(ws, &rec.id, &stem)
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    std::fs::create_dir_all(&local)?;
    if let Some(report_bytes) = &ran.report {
        std::fs::write(local.join("weft-report.json"), report_bytes)?;
    }
    let (commit, error, decided) = decide(ws, tool, (rec, subject), spec, &ran, &source_hex, now)?;
    let mut assessed = Assessed {
        gate: d.id.clone(),
        version: d.version,
        definition: d.digest.clone(),
        origin: d.origin.name().into(),
        path: d.path.clone(),
        run: rec.id.clone(),
        repo: subject.name.clone(),
        commit,
        weft: spec.weft.clone(),
        source: format!("sha256:{source_hex}"),
        pin: spec.pin.clone(),
        binary: ran.binary.clone(),
        report: ran
            .report
            .as_ref()
            .map(|r| format!("sha256:{}", crate::index::hex(&Sha256::digest(r)))),
        assumptions: decided.assumptions,
        checks: decided.checks,
        missing: decided.missing,
        outcome: writfold_core::gate::Outcome::Unknown,
        error,
        duration_ms: ran.duration_ms,
        evaluator: crate::gate::receipt::evaluator(),
        recorded: time::stamp(now),
    };
    assessed.outcome = assessed.derived();
    let path = format!("{run_dir}/{}/{stem}.json", receipt::DIR);
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let ops = [Op::Create {
        path: path.clone(),
        bytes: receipt::bytes(&assessed),
    }];
    atomic::apply(&ws.root, &state, &ops, request.probe)?;
    answer(
        ws, tool, request, rec, subject, &assessed, &path, &local, now,
    )
}

#[allow(clippy::too_many_arguments)]
fn answer(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    rec: &Rec,
    subject: &Subject,
    a: &Assessed,
    path: &str,
    local: &std::path::Path,
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let filings = Filings::read(&ws.root, &runs::dir(rec), &rec.id);
    let report = status::judge(&session, subject, &filings)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let mut answer = Answer {
        rows: status::rows(git, subject, &report, &filings)?
            .into_iter()
            .filter(|r| r.kind == "gate" && r.id == a.gate)
            .collect(),
        ..Answer::default()
    };
    let state = report
        .gates
        .iter()
        .find(|j| j.definition.id == a.gate)
        .map_or("unknown", |j| j.state.name());
    if state != "satisfied" {
        answer.counts.push((
            if state == "failed" {
                "failed"
            } else {
                "unknown"
            },
            1,
        ));
    }
    answer.notes.push(format!(
        "recorded {path} ({} in {} ms)",
        a.outcome.name(),
        a.duration_ms
    ));
    if let Some(why) = &a.error {
        answer.notes.push(format!("no assessment: {why}"));
    }
    answer.notes.push(format!(
        "Weft report (local, untracked): {}",
        local.join("weft-report.json").display()
    ));
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let frame = crate::gate::frame(request, "gate-run", vec![a.gate.clone(), rec.id.clone()]);
    session.emit(&answer, &frame)
}
