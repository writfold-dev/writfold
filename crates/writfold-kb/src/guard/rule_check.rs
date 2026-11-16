//! The checks of a guard rule file ([`super::rules`]). Every check runs
//! and reports at the position of what it judges.
use super::rule_lists::{commands, repos, requirements, strings};
use super::rules::{Event, Rule, SCHEMA};
use crate::defs::{Check, Field};
use crate::gate::def_check::{description, globs, schema};
use writfold_core::diag::Report;
use writfold_core::guard::{Action, Mode, Repeat};

const KEYS: [&str; 12] = [
    "mode",
    "schema",
    "id",
    "event",
    "commands",
    "tools",
    "paths",
    "repos",
    "require",
    "action",
    "description",
    "repeat",
];
const PRE: [&str; 11] = [
    "schema",
    "id",
    "event",
    "commands",
    "tools",
    "paths",
    "repos",
    "require",
    "action",
    "mode",
    "description",
];
const STOP: [&str; 7] = [
    "schema",
    "id",
    "event",
    "repos",
    "require",
    "repeat",
    "description",
];

fn event(c: &mut Check<'_>) -> Option<Event> {
    let event = c.need_text("event", "event = \"pre-tool-use\"")?;
    match event.str() {
        "pre-tool-use" => Some(Event::PreToolUse),
        "stop" => Some(Event::Stop),
        other => {
            let why = format!("unknown event {other:?}");
            let set = ["pre-tool-use", "stop"];
            c.unknown_value(event.span, why, "not an event", other, &set, '"');
            None
        }
    }
}

/// One of a closed set of words, for the event that takes it; reported
/// as not here for the other event.
fn word<T>(
    c: &mut Check<'_>,
    key: &str,
    wanted: bool,
    set: &[&str],
    parse: fn(&str) -> Option<T>,
) -> Option<Option<T>> {
    if !wanted {
        let (who, label) = match key {
            "action" => (
                "a stop rule",
                "a stop rule takes no action: it only adds context",
            ),
            "mode" => ("a stop rule", "mode applies to a pre-tool-use rule only"),
            _ => ("a pre-tool-use rule", "repeat applies to a stop rule only"),
        };
        let why = format!("`{key}` is not used by {who}");
        return (!c.not_here(key, why, label)).then_some(None);
    }
    match c.text(key) {
        Field::Absent => Some(None),
        Field::Bad => None,
        Field::Got(v) => match parse(v.str()) {
            Some(found) => Some(Some(found)),
            None => {
                let why = format!("unknown {key} {:?}", v.str());
                let article = if key.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    "an"
                } else {
                    "a"
                };
                c.unknown_value(
                    v.span,
                    why,
                    &format!("not {article} {key}"),
                    v.str(),
                    set,
                    '"',
                );
                None
            }
        },
    }
}

/// What a rule acts on: its commands and tools, which its event decides.
fn acts_on(c: &mut Check<'_>, event: Event, commands: &[Vec<String>], tools: &[String]) -> bool {
    match event {
        Event::PreToolUse if commands.is_empty() && tools.is_empty() => {
            c.missing(
                "commands",
                "a pre-tool-use rule",
                "commands = [\"git commit\"]   (or tools = [\"Edit\"])",
            );
            false
        }
        Event::Stop => {
            let mut ok = true;
            for key in ["commands", "tools"] {
                let why = format!("`{key}` is not used by a stop rule");
                ok &= !c.not_here(key, why, "a stop rule names no commands or tools");
            }
            ok
        }
        Event::PreToolUse => true,
    }
}

/// The `paths` key: globs over the edited file, for a rule that names
/// tools and no commands, with at least one pattern that is not an
/// exclusion.
fn paths(c: &mut Check<'_>, commands: &[Vec<String>], tools: &[String]) -> Option<Vec<String>> {
    let paths = strings(c, "paths")?;
    if paths.is_empty() {
        return Some(Vec::new());
    }
    if !commands.is_empty() || tools.is_empty() {
        let why = "`paths` applies to a rule that names tools and no commands".into();
        return (!c.not_here("paths", why, "this rule names commands")).then(Vec::new);
    }
    if !globs(c, &paths, "paths") {
        return None;
    }
    if paths.iter().all(|p| p.str().starts_with('!')) {
        let span = paths.first().map(|p| p.span);
        let why = "`paths` names at least one pattern without !".into();
        c.error("value.form", span, why).label("only exclusions");
        return None;
    }
    Some(paths.into_iter().map(|p| p.value.text).collect())
}

/// Parse and check one rule file whose name is `<stem>.toml`.
pub fn check(text: &str, stem: &str, path: &str) -> Result<Rule, Report> {
    let mut c = Check::read(path, text, "a guard rule");
    let written = c.doc.get("event").map(|e| format!("{:?}", e.value));
    let candidates: &[&str] = match written {
        Some(w) if w.contains("\"stop\"") => &STOP,
        Some(w) if w.contains("\"pre-tool-use\"") => &PRE,
        _ => &KEYS,
    };
    c.unknown_keys(&KEYS, candidates, "not a guard rule key");
    schema(&mut c, SCHEMA);
    let id = c.need_text("id", &format!("id = \"{stem}\""));
    let id = id.filter(|id| c.named("id", id, stem));
    let event = event(&mut c);
    let commands = commands(&mut c);
    let tools =
        strings(&mut c, "tools").map(|t| t.into_iter().map(|t| t.value.text).collect::<Vec<_>>());
    let pre = event == Some(Event::PreToolUse);
    let action = word(
        &mut c,
        "action",
        pre || event.is_none(),
        &["deny", "ask"],
        Action::parse,
    );
    let repeat = word(&mut c, "repeat", !pre, &["session", "turn"], Repeat::parse);
    let mode = word(
        &mut c,
        "mode",
        pre || event.is_none(),
        &["enforce", "audit"],
        Mode::parse,
    );
    let repos = repos(&mut c);
    let require = requirements(&mut c);
    let description = description(&mut c);
    let (Some(event), Some(commands), Some(tools)) = (event, commands, tools) else {
        return c.finish(None);
    };
    let acts = acts_on(&mut c, event, &commands, &tools);
    let paths = paths(&mut c, &commands, &tools);
    let action = match (event, action) {
        (Event::PreToolUse, Some(None)) => {
            c.missing(
                "action",
                "a pre-tool-use rule",
                "action = \"deny\"   (or \"ask\")",
            );
            None
        }
        (Event::PreToolUse, Some(Some(a))) => Some(a),
        (Event::Stop, _) => Some(Action::Ask),
        (_, None) => None,
    };
    let rule = (|| {
        acts.then_some(())?;
        Some(Rule {
            id: id?.value.text,
            path: path.to_owned(),
            event,
            commands,
            tools,
            paths: paths?,
            repos: repos?,
            require: require?,
            action: action?,
            repeat: repeat?.unwrap_or(Repeat::Session),
            mode: mode?.unwrap_or(Mode::Enforce),
            description: description?.unwrap_or_default(),
        })
    })();
    c.finish(rule)
}
