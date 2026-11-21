use proc_macro2::Span;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

pub struct Audit<'a> {
    pub path: &'a str,
    pub package: &'a str,
    pub file: &'a Path,
    pub package_root: &'a Path,
    pub owned_sources: &'a BTreeSet<(&'a str, &'a Path)>,
    pub lines: &'a BTreeSet<usize>,
    pub diagnostics: &'a mut Vec<Value>,
    pub definitions: &'a BTreeSet<String>,
    pub test: bool,
}

impl Audit<'_> {
    pub fn emit(&mut self, rule: &str, span: Span, message: impl Into<String>) {
        self.diagnostics.push(json!({"rule":rule,"path":self.path,
            "line":span.start().line,"column":span.start().column + 1,
            "end_line":span.end().line,"end_column":span.end().column + 1,
            "message":message.into(),"severity":"error"}));
    }

    pub fn budget(&mut self, span: Span, limit: usize, warning: usize, rule: &str) {
        let count = self
            .lines
            .range(span.start().line..=span.end().line)
            .count();
        if count > warning {
            self.diagnostics.push(json!({"rule":rule,"path":self.path,
                "line":span.start().line,"lines":count,"limit":limit,
                "severity":if count > limit {"error"} else {"warning"}}));
        }
    }

    pub fn module_path(&mut self, span: Span, value: Option<&str>) {
        let inspected = value.is_some_and(|value| {
            let Some(parent) = self.file.parent() else {
                return false;
            };
            let mut resolved = parent.to_path_buf();
            for component in Path::new(value).components() {
                match component {
                    Component::Normal(name) => resolved.push(name),
                    Component::CurDir => {}
                    Component::ParentDir => {
                        if !resolved.pop() || !resolved.starts_with(self.package_root) {
                            return false;
                        }
                    }
                    Component::RootDir | Component::Prefix(_) => return false,
                }
            }
            self.owned_sources
                .contains(&(self.package, resolved.as_path()))
        });
        if !inspected {
            self.emit(
                "opaque-module",
                span,
                "module path must name inspected source in its owning package",
            );
        }
    }
}
