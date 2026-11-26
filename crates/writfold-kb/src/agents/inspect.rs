//! `writfold kb agent inspect <id> [--repo DIR] [--json]` (#144): an agent
//! profile's resolved closure and its digest, or the first reason it does
//! not hold together. Read-only: nothing is recorded and nothing runs.
//! Exit 0 admitted, 3 refused, 1 for a usage or IO error.
//!
//! The closure digest covers behaviour only: the profile's packet rule,
//! model hint and instruction bytes, then each skill in listed order with
//! its instruction bytes and plan, then the granted tools sorted. Comments
//! and descriptions are not part of it.
use super::invocation::Used;
use super::refs::{self, Ref, Table};
use super::{Def, DefKind, Instructions};
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::gate::def::Origin;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use writfold_core::compose::Refusal;

/// The root holding a `.writfold/<kind>/` directory.
pub fn top_of(dir: &Path) -> PathBuf {
    let up = dir.parent().and_then(Path::parent);
    up.map_or_else(|| dir.to_path_buf(), Path::to_path_buf)
}

/// Every definition file in the KB root and the repository in view.
fn table(kb: &Path, cwd: &Path, repo: Option<&Path>) -> Result<Table> {
    let repo = match repo {
        Some(r) if cwd.join(r).is_dir() => Some(cwd.join(r)),
        Some(r) => {
            return Err(KbError::Usage(format!(
                "--repo {}: not a directory",
                r.display()
            )));
        }
        None => crate::defs::lint::enclosing(kb, cwd),
    };
    Ok(table_at(kb, repo.as_deref()))
}

/// Every definition file in the KB root and, when given, the repository
/// whose root is `top` (once, when that repository is the KB itself).
pub fn table_at(kb: &Path, top: Option<&Path>) -> Table {
    let mut all = crate::defs::lint::files(kb, Origin::Kb, "");
    let same = |t: &Path| std::fs::canonicalize(t).ok() == std::fs::canonicalize(kb).ok();
    if let Some(top) = top.filter(|t| !same(t)) {
        let name = (top.file_name()).map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        all.extend(crate::defs::lint::files(
            top,
            Origin::Repo,
            &format!("{name}:"),
        ));
    }
    refs::table(&all)
}

/// The bytes of a definition's instructions.
fn instruction_bytes(d: &Def) -> std::result::Result<Vec<u8>, String> {
    match &d.instructions {
        Instructions::None => Ok(Vec::new()),
        Instructions::Inline(text) => Ok(text.as_bytes().to_vec()),
        Instructions::File(rel) => crate::gate::places::resolve(&d.top, rel)
            .and_then(|real| std::fs::read(real).map_err(|e| format!("{rel}: {e}"))),
    }
}

/// The closure digest of an admitted profile.
pub fn digest(t: &Table, p: &Def) -> std::result::Result<String, String> {
    let mut h = Sha256::new();
    let mut field = |name: &str, bytes: &[u8]| {
        h.update(name.as_bytes());
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    };
    field("packet", if p.packet { b"required" } else { b"none" });
    field("model", p.model.as_deref().unwrap_or("").as_bytes());
    field("instructions", &instruction_bytes(p)?);
    // `needs` joins only when written, so a file without it keeps its digest.
    for need in &p.needs {
        field("need", need.as_bytes());
    }
    for s in p.skills.iter().filter_map(|s| t.find(DefKind::Skill, s)) {
        field("skill", s.id.as_bytes());
        field("instructions", &instruction_bytes(s)?);
        field("plan", s.plan.as_deref().unwrap_or("").as_bytes());
        for need in &s.needs {
            field("need", need.as_bytes());
        }
    }
    let mut tools = p.tools.clone();
    tools.sort();
    for tool in &tools {
        field("tool", tool.as_bytes());
    }
    Ok(format!("sha256:{}", crate::index::hex(&h.finalize())))
}

/// Why the closure was refused, from the kernel's first fault.
fn refusal(refs: &[Ref], required: &[Vec<u8>], r: Refusal) -> String {
    match r {
        Refusal::Reference(i) => refs.get(i).map_or_else(String::new, |r| {
            let why = refs::fault(r).map(|(_, why)| why).unwrap_or_default();
            format!("{} (`{}` in {})", why, r.key, r.from)
        }),
        Refusal::Tool(i) => required.get(i).map_or_else(String::new, |t| {
            let t = String::from_utf8_lossy(t);
            format!("a listed skill needs tool {t:?}, and the profile's `tools` does not grant it")
        }),
    }
}

/// Why profile `id` cannot be found as one readable definition.
fn absent(t: &Table, id: &str) -> String {
    match t.fact(DefKind::Agent, id) {
        writfold_core::compose::RefFact::Twice => {
            format!("agent {id:?} is defined in both the KB root and the repository")
        }
        _ if (t.broken.iter()).any(|(k, s)| *k == DefKind::Agent && s == id) => {
            format!("agent {id:?} has errors; run writfold kb lint")
        }
        _ => format!("no agent profile {id:?} in {}", super::AGENTS),
    }
}

/// A profile with its references (when one was found), and its closure
/// digest or why the closure is refused.
pub type Decided<'a> = (
    Option<(&'a Def, Vec<Ref>)>,
    std::result::Result<String, String>,
);

/// Profile `id` with its references, and its closure digest or why the
/// closure is refused ([`writfold_core::compose::closure`]).
pub fn decide<'a>(t: &'a Table, id: &str) -> Decided<'a> {
    let Some(p) = t.find(DefKind::Agent, id) else {
        return (None, Err(absent(t, id)));
    };
    let (refs, required, granted) = refs::facts(t, p);
    let decided = match refs::decide(&refs, &required, &granted) {
        Ok(()) => digest(t, p),
        Err(r) => Err(refusal(&refs, &required, r)),
    };
    (Some((p, refs)), decided)
}

/// The files an admitted closure was read from: the profile, its skills in
/// listed order, its granted tools, then each instructions file; each
/// with the digest of its bytes now.
pub fn used(t: &Table, p: &Def) -> std::result::Result<Vec<Used>, String> {
    let skills = p.skills.iter().filter_map(|s| t.find(DefKind::Skill, s));
    let tools = p.tools.iter().filter_map(|s| t.find(DefKind::Tool, s));
    let defs: Vec<&Def> = std::iter::once(p).chain(skills).chain(tools).collect();
    let mut out = Vec::new();
    for d in &defs {
        let rel = format!("{}/{}.toml", d.kind.dir(), d.id);
        let bytes = std::fs::read(d.top.join(&rel)).map_err(|e| format!("{}: {e}", d.shown))?;
        let shown = d.shown.clone();
        out.push(Used {
            kind: d.kind.word().to_owned(),
            id: d.id.clone(),
            path: shown,
            digest: super::invocation::sha(&bytes),
        });
    }
    for d in &defs {
        if let Instructions::File(rel) = &d.instructions {
            let prefix = d.shown.split_once(':').map_or("", |(repo, _)| repo);
            out.push(Used {
                kind: "instructions".into(),
                id: d.id.clone(),
                path: match prefix.is_empty() {
                    true => rel.clone(),
                    false => format!("{prefix}:{rel}"),
                },
                digest: super::invocation::sha(&instruction_bytes(d)?),
            });
        }
    }
    Ok(out)
}

/// The instructions a worker reads: the profile's, then each skill's in
/// listed order, each under a heading.
pub fn instructions(t: &Table, p: &Def) -> std::result::Result<Vec<u8>, String> {
    let mut out = format!("# Agent {}\n\n", p.id).into_bytes();
    out.extend(instruction_bytes(p)?);
    for s in p.skills.iter().filter_map(|s| t.find(DefKind::Skill, s)) {
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend(format!("\n# Skill {}\n\n", s.id).into_bytes());
        out.extend(instruction_bytes(s)?);
    }
    Ok(out)
}

/// `agent inspect` for profile `id`.
pub fn run(kb: &Path, request: &crate::Request, id: &str, repo: Option<&Path>) -> Result<Output> {
    let t = table(kb, &request.cwd, repo)?;
    let (found, decided) = decide(&t, id);
    let found = found.as_ref().map(|(p, refs)| (*p, refs.as_slice()));
    Ok(render(id, found, decided, request.json))
}

fn render(
    id: &str,
    found: Option<(&Def, &[Ref])>,
    decided: std::result::Result<String, String>,
    json: bool,
) -> Output {
    let rows: Vec<(String, String, String)> = found.map_or_else(Vec::new, |(p, refs)| {
        let mut rows = vec![("agent".to_owned(), p.id.clone(), p.shown.clone())];
        rows.extend(refs.iter().map(|r| {
            let at = format!("{} via `{}` in {}", word(r.fact), r.key, r.from);
            (r.what.to_owned(), r.id.clone(), at)
        }));
        rows
    });
    let (answer, exit) = match &decided {
        Ok(_) => ("admitted", 0),
        Err(_) => ("refused", 3),
    };
    let text = match json {
        true => {
            let rows: Vec<serde_json::Value> = (rows.iter())
                .map(|(k, i, at)| json!({"kind": k, "id": i, "at": at}))
                .collect();
            let (digest, why) = match &decided {
                Ok(d) => (json!(d), json!(null)),
                Err(w) => (json!(null), json!(w)),
            };
            let trailer =
                json!({"coverage": "complete", "answer": answer, "digest": digest, "refused": why});
            format!(
                "{}\n",
                json!({"writfold_kb": 1, "verb": "agent inspect", "agent": id, "rows": rows, "trailer": trailer})
            )
        }
        false => {
            let mut out: String = (rows.iter())
                .map(|(k, i, at)| format!("{k} {i} {at}\n"))
                .collect();
            match &decided {
                Ok(d) => out.push_str(&format!("# closure={d}\n")),
                Err(w) => out.push_str(&format!("# refused: {w}\n")),
            }
            out.push_str(&format!(
                "# note: read only; nothing was recorded\n# coverage=complete answer={answer}\n"
            ));
            out
        }
    };
    Output {
        text,
        exit,
        report: None,
        shown: None,
        drawn: None,
    }
}

fn word(f: writfold_core::compose::RefFact) -> &'static str {
    use writfold_core::compose::RefFact;
    match f {
        RefFact::Found => "found",
        RefFact::Missing => "missing",
        RefFact::WrongKind => "wrong-kind",
        RefFact::Twice => "twice",
    }
}
