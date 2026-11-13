//! Checker: resolves bindings in source order (no shadowing), assigns each a
//! kind, refuses deferred operators by type, and lowers the one pipeline that
//! `emit` reaches into the IR. It reports every diagnostic it finds.
use crate::ast::{self, FieldKind, Matcher, Name, Number, Operation, Program};
use crate::diag::{Code, Deferred, Diagnostic, Span};
use crate::ir::{self, EmitPlan, Field, FindPlan, MatcherPlan, Plan, RetainPlan, ViewPlan};
use crate::locate::Pattern;
use std::collections::BTreeMap;
use writfold_domain::citation::SearchBudget;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Hits,
    Evidence,
    View,
    /// Produced by a refused operation; suppresses follow-on diagnostics.
    Poisoned,
}

impl Kind {
    fn noun(self) -> &'static str {
        match self {
            Self::Hits => "hits",
            Self::Evidence => "retained evidence",
            Self::View => "a view",
            Self::Poisoned => "a refused result",
        }
    }
}

struct Entry {
    kind: Kind,
    index: usize,
    used: bool,
    span: Span,
}

#[derive(Default)]
struct Checker {
    names: BTreeMap<String, Entry>,
    diagnostics: Vec<Diagnostic>,
    plans: BTreeMap<usize, Lowered>,
    allow_refs: bool,
}

enum Lowered {
    Find(FindPlan),
    Retain(String, RetainPlan),
    View(String, ViewPlan),
}

pub fn check(program: &Program, source: &[u8]) -> Result<Plan, Vec<Diagnostic>> {
    check_mode(program, source, false)
}

pub(crate) fn check_mode(
    program: &Program,
    source: &[u8],
    allow_refs: bool,
) -> Result<Plan, Vec<Diagnostic>> {
    let mut checker = Checker {
        allow_refs,
        ..Checker::default()
    };
    for (index, binding) in program.bindings.iter().enumerate() {
        checker.binding(index, binding);
    }
    let emitted = checker.reference(&program.emit.source, &[Kind::View]);
    let budget = checker.bounded(
        program.emit.budget,
        ir::DEFAULT_BUDGET_TOKENS,
        1,
        ir::MAX_BUDGET_TOKENS,
        "budget_tokens",
    );
    for (name, entry) in &checker.names {
        if !entry.used && entry.kind != Kind::Poisoned {
            checker.diagnostics.push(Diagnostic::new(
                Code::Unused,
                entry.span,
                format!("binding @{name} is never used"),
            ));
        }
    }
    if !checker.diagnostics.is_empty() {
        checker.diagnostics.sort_by_key(|d| d.span);
        return Err(checker.diagnostics);
    }
    checker
        .lower(emitted, budget, crate::digest_hex(source))
        .ok_or_else(|| {
            vec![Diagnostic::new(
                Code::Type,
                program.emit.span,
                "emit does not reach a find through project",
            )]
        })
}

impl Checker {
    fn error(&mut self, code: Code, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(code, span, message));
    }

    /// Resolve a reference and mark it used; `None` if it is unknown or of a
    /// kind outside `admitted` (both reported here).
    fn reference(&mut self, name: &Name, admitted: &[Kind]) -> Option<(Kind, usize)> {
        let Some(entry) = self.names.get_mut(&name.text) else {
            self.error(
                Code::UnknownBinding,
                name.span,
                format!("@{} is not bound earlier", name.text),
            );
            return None;
        };
        entry.used = true;
        let (kind, index) = (entry.kind, entry.index);
        if kind == Kind::Poisoned {
            return None;
        }
        if !admitted.contains(&kind) {
            let wanted: Vec<_> = admitted.iter().map(|k| k.noun()).collect();
            self.error(
                Code::Type,
                name.span,
                format!(
                    "@{} is {}, expected {}",
                    name.text,
                    kind.noun(),
                    wanted.join(" or ")
                ),
            );
            return None;
        }
        Some((kind, index))
    }

    fn bounded(&mut self, n: Option<Number>, default: u64, lo: u64, hi: u64, what: &str) -> usize {
        let Some(n) = n else {
            return default as usize;
        };
        if n.value < lo || n.value > hi {
            self.error(
                Code::Range,
                n.span,
                format!("{what} must be {lo}..={hi}, found {}", n.value),
            );
        }
        n.value.min(hi) as usize
    }

    fn binding(&mut self, index: usize, binding: &ast::Binding) {
        let kind = match &binding.operation {
            Operation::Find(find) => self.find(index, &binding.name.text, find),
            Operation::Retain { source, around } => {
                self.retain(index, &binding.name.text, source, *around)
            }
            Operation::Project(project) => self.project(index, &binding.name.text, project),
            Operation::Rank { source, .. } => self.deferred(Deferred::Rank, binding.span, source),
            Operation::Follow { source, .. } => {
                self.deferred(Deferred::Follow, binding.span, source)
            }
        };
        if let Some(previous) = self.names.get(&binding.name.text) {
            let message = format!(
                "@{} is already bound at byte {}; bindings are never shadowed",
                binding.name.text, previous.span.lo
            );
            self.error(Code::DuplicateBinding, binding.name.span, message);
            return;
        }
        self.names.insert(
            binding.name.text.clone(),
            Entry {
                kind,
                index,
                used: false,
                span: binding.name.span,
            },
        );
    }

    fn deferred(&mut self, which: Deferred, span: Span, source: &Name) -> Kind {
        self.reference(source, &[Kind::Hits, Kind::Evidence, Kind::View]);
        self.unsupported(which, span);
        Kind::Poisoned
    }

    fn unsupported(&mut self, which: Deferred, span: Span) {
        let message = format!(
            "`{}` needs the Omnigraph backend (P01O/WF24), which is blocked; it is refused, never emulated",
            which.keyword()
        );
        self.error(Code::Unsupported(which), span, message);
    }

    fn find(&mut self, index: usize, name: &str, find: &ast::Find) -> Kind {
        let matcher = match &find.matcher {
            Matcher::Similar(_) => {
                self.unsupported(Deferred::Similar, find.matcher_span);
                return Kind::Poisoned;
            }
            Matcher::Text(needle) => {
                if needle.is_empty() || needle.len() > ir::MAX_NEEDLE_BYTES {
                    let message = format!("text needle must be 1..={} bytes", ir::MAX_NEEDLE_BYTES);
                    self.error(Code::Needle, find.matcher_span, message);
                }
                MatcherPlan::Text(needle.clone())
            }
            Matcher::Rx(source) => match Pattern::compile(source) {
                Ok(pattern) => MatcherPlan::Rx(pattern),
                Err(message) => {
                    self.error(Code::Regex, find.matcher_span, message);
                    return Kind::Hits;
                }
            },
            Matcher::Symbol { kind, name } => {
                if name.is_empty() || name.len() > ir::MAX_NEEDLE_BYTES {
                    self.error(
                        Code::Needle,
                        find.matcher_span,
                        "symbol name must be 1..=4096 bytes",
                    );
                }
                MatcherPlan::Symbol {
                    kind: *kind,
                    name: name.clone(),
                }
            }
        };
        let scope = find
            .scope
            .as_ref()
            .map_or(String::new(), |s| s.value.clone());
        if let Some(text) = &find.scope
            && let Err(message) = admit_scope(&text.value)
        {
            self.error(Code::Scope, text.span, message);
        }
        self.pin(&find.at);
        let limit = self.bounded(
            find.limit,
            ir::DEFAULT_FIND_LIMIT,
            1,
            ir::MAX_FIND_LIMIT,
            "find limit",
        );
        let plan = FindPlan {
            binding: name.to_owned(),
            matcher,
            scope,
            commit: find.at.value.clone(),
            budget: SearchBudget::new(limit, limit),
        };
        self.plans.insert(index, Lowered::Find(plan));
        Kind::Hits
    }

    fn pin(&mut self, at: &ast::Text) {
        if self.allow_refs && !at.value.is_empty() {
            return;
        }
        if !full_commit(&at.value) {
            let base = "`at` needs a full 40- or 64-digit lowercase commit id; refs, abbreviations and HEAD are refused";
            let value = &at.value;
            let message = if value.is_empty() || value.contains(['\'', '\n']) {
                base.to_owned()
            } else {
                format!("{base}; pin it with `git rev-parse '{value}^{{commit}}'`")
            };
            self.error(Code::Pin, at.span, message);
        }
    }

    fn retain(&mut self, index: usize, name: &str, source: &Name, around: Option<Number>) -> Kind {
        let around =
            around.map(|n| self.bounded(Some(n), 0, 0, ir::MAX_AROUND_LINES, "around lines"));
        let plan = RetainPlan {
            binding: name.to_owned(),
            around,
        };
        if self.reference(source, &[Kind::Hits]).is_some() {
            self.plans
                .insert(index, Lowered::Retain(source.text.clone(), plan));
        }
        Kind::Evidence
    }

    fn project(&mut self, index: usize, name: &str, project: &ast::Project) -> Kind {
        let source = self.reference(&project.source, &[Kind::Hits, Kind::Evidence]);
        let evidence = source.is_none_or(|(kind, _)| kind == Kind::Evidence);
        let mut fields = Vec::new();
        for field in &project.fields {
            let lowered = match field.kind {
                FieldKind::Path => Field::Path,
                FieldKind::Line => Field::Line,
                FieldKind::Range => Field::Range,
                FieldKind::Binding => Field::Binding,
                FieldKind::Excerpt(max) => Field::Excerpt {
                    max_chars: self.bounded(Some(max), 1, 1, ir::MAX_EXCERPT_CHARS, "max_chars"),
                },
            };
            if fields.iter().any(|f: &Field| f.name() == lowered.name()) {
                self.error(
                    Code::Field,
                    field.span,
                    format!("field `{}` repeated", lowered.name()),
                );
            }
            if !evidence && matches!(lowered, Field::Binding | Field::Excerpt { .. }) {
                let message = format!(
                    "field `{}` needs retained evidence; add `@e := retain @{};` before `@{name} := project` and project @e instead",
                    lowered.name(),
                    project.source.text
                );
                self.error(Code::Type, field.span, message);
            }
            fields.push(lowered);
        }
        let mut sort = Vec::new();
        for key in &project.sort {
            if sort.iter().any(|(f, _)| *f == key.field) {
                self.error(Code::Field, key.span, "sort key repeated");
            }
            sort.push((key.field, key.direction));
        }
        let limit = project
            .limit
            .map(|n| self.bounded(Some(n), 1, 1, ir::MAX_VIEW_LIMIT, "project limit"));
        let plan = ViewPlan {
            binding: name.to_owned(),
            evidence,
            fields,
            sort,
            limit,
        };
        self.plans
            .insert(index, Lowered::View(project.source.text.clone(), plan));
        Kind::View
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.names.get(name).map(|entry| entry.index)
    }

    fn lower(
        mut self,
        emitted: Option<(Kind, usize)>,
        budget: usize,
        program: String,
    ) -> Option<Plan> {
        let (_, view_index) = emitted?;
        let Some(Lowered::View(source, view)) = self.plans.remove(&view_index) else {
            return None;
        };
        let (retain, find_index) = match self.plans.remove(&self.index_of(&source)?)? {
            Lowered::Retain(hits, retain) => (Some(retain), self.index_of(&hits)?),
            Lowered::Find(find) => return Some(plan(program, find, None, view, budget)),
            Lowered::View(..) => return None,
        };
        match self.plans.remove(&find_index)? {
            Lowered::Find(find) => Some(plan(program, find, retain, view, budget)),
            _ => None,
        }
    }
}

fn plan(
    program: String,
    find: FindPlan,
    retain: Option<RetainPlan>,
    view: ViewPlan,
    budget: usize,
) -> Plan {
    Plan {
        program,
        find,
        retain,
        view,
        emit: EmitPlan {
            budget_tokens: budget,
        },
    }
}

pub(crate) fn full_commit(text: &str) -> bool {
    matches!(text.len(), 40 | 64)
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn admit_scope(scope: &str) -> Result<(), String> {
    if scope.len() > ir::MAX_SCOPE_BYTES {
        return Err(format!(
            "scope must be at most {} bytes",
            ir::MAX_SCOPE_BYTES
        ));
    }
    if scope.starts_with('/') {
        return Err("scope is a repository-relative path prefix; a leading `/` is refused".into());
    }
    if scope
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err("`.` and `..` segments are refused in a scope".into());
    }
    Ok(())
}
