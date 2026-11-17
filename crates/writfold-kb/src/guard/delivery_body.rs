//! What a guarded call would post, read from the call without running it:
//! a route's body flags, file flags or tool field; a listed command's
//! `--body`/`-b`/`--body-file`/`-F`; or nothing (a listed tool).
use crate::deliver::config::Guard;
use crate::deliver::guarded::Read;
use crate::deliver::route::Route;
use crate::shell::{self, Dir, Word};
use serde_json::{Map, Value};
use std::path::Path;
use writfold_core::delivery::Unread;

/// The body flags a listed command is read by, with whether each names a
/// file.
const BODY_FLAGS: [(&str, bool); 4] = [
    ("--body", false),
    ("-b", false),
    ("--body-file", true),
    ("-F", true),
];

/// A guarded call, matched.
pub struct Matched {
    pub name: String,
    /// The mode a route names.
    pub mode: Option<String>,
    /// The flag that posts a file, for a denial that names the repair.
    pub file_flag: Option<String>,
    /// Matched by a route (an unread body is denied).
    pub routed: bool,
    pub read: Read,
}

impl Matched {
    pub fn unread(&self) -> Unread {
        match self.routed {
            true => Unread::Deny,
            false => Unread::AllReady,
        }
    }
}

fn routed(r: &Route, read: Read) -> Matched {
    Matched {
        name: r.name(),
        mode: r.mode.clone(),
        file_flag: r.file.first().cloned(),
        routed: true,
        read,
    }
}

/// The call `input` makes, when the guard covers it.
pub fn matched(guard: &Guard, input: &Map<String, Value>, cwd: &Path) -> Option<Matched> {
    let tool = input.get("tool_name").and_then(Value::as_str).unwrap_or("");
    if let Some(r) = guard
        .routes
        .iter()
        .find(|r| r.is_tool() && r.name() == tool)
    {
        let field = r.field.as_deref().unwrap_or("");
        let text = input
            .get("tool_input")
            .and_then(|t| t.get(field))
            .and_then(Value::as_str);
        let read = match text {
            Some(t) => Read::Bytes(t.as_bytes().to_vec()),
            None => Read::Unread(format!("the tool input has no string at {field}")),
        };
        return Some(routed(r, read));
    }
    if guard.tools.iter().any(|t| t == tool) {
        return Some(Matched {
            name: tool.to_owned(),
            mode: None,
            file_flag: None,
            routed: false,
            read: Read::Unread(format!("tool {tool} is guarded as a whole")),
        });
    }
    if tool != "Bash" {
        return None;
    }
    let line = input
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    bash(guard, line, cwd)
}

fn bash(guard: &Guard, line: &str, cwd: &Path) -> Option<Matched> {
    let home = std::env::var("HOME").ok();
    let cwd_text = cwd.to_string_lossy();
    let here = shell::invocations("true", &cwd_text, home.as_deref())
        .first()
        .map(|i| i.dir.clone());
    let invocations = shell::invocations(line, &cwd_text, home.as_deref());
    for (segment, inv) in shell::segments(line).iter().zip(&invocations) {
        let words = shell::command(segment);
        let elsewhere = Some(&inv.dir) != here.as_ref() || matches!(inv.dir, Dir::Unresolved(_));
        if let Some(r) = guard
            .routes
            .iter()
            .find(|r| !r.is_tool() && shell::matches(&r.words, &inv.words))
        {
            let flags: Vec<(&str, bool)> = (r.body.iter().map(|f| (f.as_str(), false)))
                .chain(r.file.iter().map(|f| (f.as_str(), true)))
                .collect();
            return Some(routed(r, read(words, &flags, cwd, elsewhere)));
        }
        if guard.commands.iter().any(|c| shell::matches(c, &inv.words)) {
            return Some(Matched {
                name: inv
                    .words
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
                mode: None,
                file_flag: Some("--body-file".into()),
                routed: false,
                read: read(words, &BODY_FLAGS, cwd, elsewhere),
            });
        }
    }
    None
}

/// The body named by exactly one of `flags` in `words`.
fn read(words: &[Word], flags: &[(&str, bool)], cwd: &Path, elsewhere: bool) -> Read {
    let mut hits: Vec<(&str, bool, Option<Word>)> = Vec::new();
    for (i, word) in words.iter().enumerate() {
        for (flag, file) in flags {
            if word.text == *flag {
                hits.push((flag, *file, words.get(i + 1).cloned()));
            } else if flag.starts_with("--")
                && let Some(value) = word.text.strip_prefix(&format!("{flag}="))
            {
                let w = Word {
                    text: value.to_owned(),
                    ..word.clone()
                };
                hits.push((flag, *file, Some(w)));
            }
        }
    }
    let (flag, file, value) = match hits.as_slice() {
        [] => return Read::Unread("no body flag the guard can read".into()),
        [one] => one,
        _ => return Read::Unread("a body flag appears more than once".into()),
    };
    let Some(value) = value else {
        return Read::Unread(format!("{flag} has no value"));
    };
    if value.expands {
        return Read::Unread("the body holds a shell expansion".into());
    }
    if !file {
        return Read::Bytes(value.text.as_bytes().to_vec());
    }
    if value.text == "-" {
        return Read::Unread("the body is read from standard input".into());
    }
    if elsewhere && !Path::new(&value.text).is_absolute() {
        return Read::Unread(
            "the command acts in another directory (cd, pushd, -C or a Git directory setting), so a relative body file is not read from here".into(),
        );
    }
    match std::fs::read(cwd.join(&value.text)) {
        Ok(bytes) => Read::Bytes(bytes),
        Err(e) => Read::Unread(format!("the body file {} is unreadable ({e})", value.text)),
    }
}
