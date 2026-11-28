//! A guard route (`guard.toml` `routes`): a posting command or tool named
//! with its delivery mode and where its body is.
//!
//! `"<command words or tool name>: key=value …"`, the form a deny rule and
//! a plan step already use. `mode` (optional) names a delivery mode that
//! must load; `body` lists flags whose value is the text; `file` flags
//! whose value is a path; `field` the key of a tool's input that holds the
//! text, which makes the route a tool route. Under a route, a body the
//! guard cannot read is denied.
use crate::defs::{Check, Spanned, Text};
use std::path::Path;

/// One route.
#[derive(Clone, Debug, Default)]
pub struct Route {
    /// The command's words, or the tool's name as one word.
    pub words: Vec<String>,
    pub mode: Option<String>,
    pub body: Vec<String>,
    pub file: Vec<String>,
    /// A tool route's input key.
    pub field: Option<String>,
    /// With a positional `file=@N`, the flags that take a value (#187).
    pub flags: Vec<String>,
}

impl Route {
    pub fn name(&self) -> String {
        self.words.join(" ")
    }

    pub fn is_tool(&self) -> bool {
        self.field.is_some()
    }
}

fn split(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `file=@N` is one positional (`@1` … `@9`, `@last`) and nothing else;
/// `flags=` only goes with it.
fn positional_ok(route: &Route) -> bool {
    let at: Vec<&String> = route.file.iter().filter(|f| f.starts_with('@')).collect();
    let well =
        |f: &str| f == "@last" || f.len() == 2 && f[1..].chars().all(|c| ('1'..='9').contains(&c));
    match at.as_slice() {
        [] => route.flags.is_empty(),
        [one] => route.file.len() == 1 && route.body.is_empty() && well(one),
        _ => false,
    }
}

/// Read one route entry, reporting each fault at its place; `taken` are
/// the words of `commands` and `tools`.
pub fn read(
    c: &mut Check<'_>,
    entry: &Spanned<Text>,
    taken: &[String],
    root: Option<&Path>,
) -> Option<Route> {
    let Some((name, rest)) = entry.str().split_once(": ") else {
        let why = format!("route {:?} is not \"command: key=value …\"", entry.str());
        c.error("value.form", Some(entry.span), why)
            .label("no `: ` after the command or tool");
        return None;
    };
    let mut route = Route {
        words: name.split_whitespace().map(str::to_owned).collect(),
        ..Route::default()
    };
    let mut ok = !route.words.is_empty();
    for pair in rest.split_whitespace() {
        let at = entry.sub(entry.str().find(pair).unwrap_or(0), pair.len());
        match pair.split_once('=') {
            Some(("mode", m)) => route.mode = Some(m.to_owned()),
            Some(("body", v)) => route.body = split(v),
            Some(("file", v)) => route.file = split(v),
            Some(("field", v)) if !v.is_empty() => route.field = Some(v.to_owned()),
            Some(("flags", v)) => route.flags = split(v),
            _ => {
                let why = format!("{pair:?} is not mode=, body=, file=, flags= or field=");
                c.error("value.form", Some(at), why)
                    .label("not a route key");
                ok = false;
            }
        }
    }
    let conflict = |why: &str| (why.to_owned(), "conflicting");
    let fault = match (&route.field, route.body.is_empty() && route.file.is_empty()) {
        (Some(_), false) => Some(conflict("a tool route (field=) takes no body= or file=")),
        (None, true) => Some(conflict("a route needs body=, file= or field=")),
        _ if !positional_ok(&route) => Some(conflict(
            "a positional body is one file=@1, @2 … or @last, alone; flags= goes with it only",
        )),
        _ if taken.contains(&route.name()) => Some(conflict(
            "this command or tool is also listed under commands or tools",
        )),
        _ => None,
    };
    if let Some((why, label)) = fault {
        c.error("value.conflict", Some(entry.span), why)
            .label(label);
        ok = false;
    }
    if let (Some(mode), Some(root)) = (&route.mode, root)
        && let Err(e) = super::mode::mode(root, mode)
    {
        let why = format!("route mode {mode} does not load: {e}");
        c.error("ref.undefined", Some(entry.span), why)
            .label("no such delivery mode");
        ok = false;
    }
    ok.then_some(route)
}
