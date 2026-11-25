//! Resolving a skill's or profile's references into the facts
//! [`writfold_core::compose`] decides on, and `kb lint`'s diagnostics for
//! them. A reference to a file that exists but has errors is not reported
//! again by the lint (its own faults are); `agent inspect` refuses on it.
use super::{Def, DefKind};
use crate::defs::lint::{File, Kind};
use crate::defs::{Sink, Slot, Value, toml};
use writfold_core::compose::{RefFact, Refusal, closure, covers};
use writfold_core::diag::{Report, Severity, Span};

/// Every tool, skill and profile in both roots, and the plan templates.
#[derive(Default)]
pub struct Table {
    pub defs: Vec<Def>,
    /// Files of the three kinds that did not parse: kind and stem.
    pub broken: Vec<(DefKind, String)>,
    pub templates: Vec<String>,
}

pub fn def_kind(kind: Kind) -> Option<DefKind> {
    match kind {
        Kind::Tool => Some(DefKind::Tool),
        Kind::Skill => Some(DefKind::Skill),
        Kind::Agent => Some(DefKind::Agent),
        _ => None,
    }
}

/// The table of every file `kb lint` read.
pub fn table(all: &[File]) -> Table {
    let mut t = Table::default();
    for f in all {
        if f.kind == Kind::Template && !t.templates.contains(&f.stem) {
            t.templates.push(f.stem.clone());
        }
        let Some(kind) = def_kind(f.kind) else {
            continue;
        };
        let top = super::inspect::top_of(&f.dir);
        match super::manifest::check(kind, &f.text, &f.stem, &f.shown, &top) {
            Ok(def) if f.report.errors() == 0 => t.defs.push(def),
            _ => t.broken.push((kind, f.stem.clone())),
        }
    }
    t
}

impl Table {
    /// What a reference to `id` as a `kind` finds.
    pub fn fact(&self, kind: DefKind, id: &str) -> RefFact {
        let same = (self.defs.iter())
            .filter(|d| d.kind == kind && d.id == id)
            .count();
        let broken = self.broken.iter().any(|(k, s)| *k == kind && s == id);
        match (same, broken) {
            (1, false) => RefFact::Found,
            (0, false) => {
                let other = self.defs.iter().any(|d| d.id == id)
                    || self.broken.iter().any(|(_, s)| s == id);
                match other {
                    true => RefFact::WrongKind,
                    false => RefFact::Missing,
                }
            }
            (0, true) => RefFact::Missing,
            _ => RefFact::Twice,
        }
    }

    pub fn find(&self, kind: DefKind, id: &str) -> Option<&Def> {
        (self.defs.iter()).find(|d| d.kind == kind && d.id == id)
    }

    fn plan_fact(&self, plan: &str) -> RefFact {
        match self.templates.iter().any(|t| t == plan) {
            true => RefFact::Found,
            false => RefFact::Missing,
        }
    }
}

/// One reference in the order the kernel reads them.
pub struct Ref {
    /// `skill`, `tool`, `plan`.
    pub what: &'static str,
    pub id: String,
    /// The definition naming it, and the key it is written under.
    pub from: String,
    pub key: &'static str,
    pub fact: RefFact,
}

/// A profile's references (its skills in order, its tools, each found
/// skill's tools and plan), the tools its skills need, and those it grants.
pub fn facts(t: &Table, p: &Def) -> (Vec<Ref>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut refs: Vec<Ref> = Vec::new();
    let mut push = |what, id: &str, from: &Def, key, fact| {
        let (id, from) = (id.to_owned(), from.shown.clone());
        refs.push(Ref {
            what,
            id,
            from,
            key,
            fact,
        });
    };
    for s in &p.skills {
        push("skill", s, p, "skills", t.fact(DefKind::Skill, s));
    }
    for tool in &p.tools {
        push("tool", tool, p, "tools", t.fact(DefKind::Tool, tool));
    }
    let mut required: Vec<Vec<u8>> = Vec::new();
    for skill in p.skills.iter().filter_map(|s| t.find(DefKind::Skill, s)) {
        for tool in &skill.tools {
            push("tool", tool, skill, "tools", t.fact(DefKind::Tool, tool));
            if !required.iter().any(|r| r == tool.as_bytes()) {
                required.push(tool.as_bytes().to_vec());
            }
        }
        if let Some(plan) = &skill.plan {
            push("plan", plan, skill, "plan", t.plan_fact(plan));
        }
    }
    let granted = p.tools.iter().map(|g| g.as_bytes().to_vec()).collect();
    (refs, required, granted)
}

/// The kernel's decision over a profile's facts.
pub fn decide(refs: &[Ref], required: &[Vec<u8>], granted: &[Vec<u8>]) -> Result<(), Refusal> {
    let facts: Vec<RefFact> = refs.iter().map(|r| r.fact).collect();
    closure(&facts, required, granted)
}

/// Why a reference fact is a fault, or `None` when it resolves.
pub fn fault(r: &Ref) -> Option<(&'static str, String)> {
    let what = r.what;
    match r.fact {
        RefFact::Found => None,
        RefFact::Missing => Some(("ref.undefined", format!("no {what} {:?} is defined", r.id))),
        RefFact::WrongKind => Some((
            "ref.wrong-kind",
            format!("{:?} is defined, but not as a {what}", r.id),
        )),
        RefFact::Twice => Some((
            "file.twice",
            format!(
                "{what} {:?} is defined in both the KB root and the repository",
                r.id
            ),
        )),
    }
}

fn key_span(text: &str, key: &str) -> Option<Span> {
    crate::defs::Check::read("", text, "a definition").key_span(key)
}

fn item_span(text: &str, key: &str, value: &str) -> Option<Span> {
    let doc = toml::read(text, &mut Sink::new("", text));
    let Some(Slot::Value(v)) = doc.get(key).map(|e| &e.value) else {
        return None;
    };
    match &v.value {
        Value::List(items) => items.iter().find(|i| i.str() == value).map(|i| i.span),
        Value::Str(_) => Some(v.span),
        Value::Int(_) => None,
    }
}

/// `kb lint`'s reference faults for the three kinds: each unresolved
/// reference in a skill or profile (not one already reported as a file in
/// both roots), and each tool a listed skill needs that the profile does
/// not grant.
pub fn lint(all: &[File]) -> Report {
    let t = table(all);
    let mut report = Report::default();
    let mut emit = |file: &File, span, (code, why): (&'static str, String), label: &str| {
        let mut sink = Sink::new(&file.shown, &file.text);
        let d = sink.push(Severity::Error, code, span, why);
        d.label = label.to_owned();
        report.extend(sink.report());
    };
    for def in &t.defs {
        let Some(file) = all.iter().find(|f| f.shown == def.shown) else {
            continue;
        };
        let (refs, required, granted) = match def.kind {
            DefKind::Agent => facts(&t, def),
            DefKind::Skill => (skill_refs(&t, def), Vec::new(), Vec::new()),
            DefKind::Tool => continue,
        };
        for r in refs
            .iter()
            .filter(|r| r.from == def.shown && r.fact != RefFact::Twice)
        {
            if let Some(fault) = fault(r) {
                emit(
                    file,
                    item_span(&file.text, r.key, &r.id),
                    fault,
                    "unresolved",
                );
            }
        }
        let mut need = required.clone();
        while let Some(i) = covers(&need, &granted) {
            let tool = String::from_utf8_lossy(&need[i]).into_owned();
            let why = format!("a listed skill needs tool {tool:?}, and `tools` does not grant it");
            emit(
                file,
                key_span(&file.text, "tools"),
                ("value.unmeetable", why),
                "not granted",
            );
            need.remove(i);
        }
    }
    report
}

fn skill_refs(t: &Table, s: &Def) -> Vec<Ref> {
    let mut refs: Vec<Ref> = (s.tools.iter())
        .map(|tool| Ref {
            what: "tool",
            id: tool.clone(),
            from: s.shown.clone(),
            key: "tools",
            fact: t.fact(DefKind::Tool, tool),
        })
        .collect();
    if let Some(plan) = &s.plan {
        refs.push(Ref {
            what: "plan",
            id: plan.clone(),
            from: s.shown.clone(),
            key: "plan",
            fact: t.plan_fact(plan),
        });
    }
    refs
}
