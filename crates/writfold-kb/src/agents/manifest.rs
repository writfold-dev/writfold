//! Reading one tool, skill or agent profile (#144, RFC
//! 2026-11-25-agent-vocabulary): every fault at its line and column, as for
//! every other file `kb lint` reads. Reading runs nothing and fetches
//! nothing; references between files are resolved by [`super::refs`].
use super::{Def, DefKind, Instructions};
use crate::defs::{Check, Field, Spanned, Text};
use crate::gate::def_check::{description, schema};
use crate::gate::places;
use std::path::Path;
use writfold_core::diag::Report;

const TOOL_KEYS: [&str; 3] = ["schema", "id", "description"];
const SKILL_KEYS: [&str; 8] = [
    "schema",
    "id",
    "description",
    "tools",
    "plan",
    "needs",
    "instructions",
    "instructions_file",
];
const AGENT_KEYS: [&str; 10] = [
    "schema",
    "id",
    "description",
    "skills",
    "tools",
    "packet",
    "model",
    "needs",
    "instructions",
    "instructions_file",
];
/// `packet`'s values; `none` is the default.
pub const PACKETS: [&str; 2] = ["none", "required"];

fn keys(kind: DefKind) -> &'static [&'static str] {
    match kind {
        DefKind::Tool => &TOOL_KEYS,
        DefKind::Skill => &SKILL_KEYS,
        DefKind::Agent => &AGENT_KEYS,
    }
}

/// A tool id: a slug, or two slugs joined by one `.` (`repo.read`).
pub fn tool_id(id: &str) -> bool {
    let mut parts = id.split('.');
    let first = parts.next().is_some_and(crate::write::is_slug);
    let second = parts.next().is_none_or(crate::write::is_slug);
    first && second && parts.next().is_none()
}

fn id_ok(kind: DefKind, id: &str) -> bool {
    match kind {
        DefKind::Tool => tool_id(id),
        DefKind::Skill | DefKind::Agent => crate::write::is_slug(id),
    }
}

/// `id`: the file's stem, in its kind's grammar.
fn id(c: &mut Check<'_>, kind: DefKind, stem: &str) -> Option<String> {
    let id = c.need_text("id", &format!("id = \"{stem}\""))?;
    if id.str() == stem && id_ok(kind, stem) {
        return Some(id.value.text);
    }
    let why = match (id_ok(kind, id.str()), kind) {
        (false, DefKind::Tool) => format!(
            "a tool id is a slug, or two joined by one `.`, and {:?} is not",
            id.str()
        ),
        (false, _) => format!(
            "an {} id is a slug (lower-case letters, digits and -), and {:?} is not",
            kind.word(),
            id.str()
        ),
        (true, _) => format!("`id` is {:?}, but the file is named {stem}.toml", id.str()),
    };
    c.error("value.form", Some(id.span), why)
        .label("not this file's id");
    None
}

/// A list of ids, each in the grammar of the kind it names.
fn ids(c: &mut Check<'_>, key: &str, of: DefKind) -> Option<Vec<String>> {
    listed(c, key, &|id| id_ok(of, id), of.word())
}

/// `needs`: gate ids (slugs), each once (#154).
fn needs(c: &mut Check<'_>) -> Option<Vec<String>> {
    listed(c, "needs", &crate::write::is_slug, "gate")
}

fn listed(
    c: &mut Check<'_>,
    key: &str,
    ok: &dyn Fn(&str) -> bool,
    word: &str,
) -> Option<Vec<String>> {
    let items = match c.list(key) {
        Field::Absent => return Some(Vec::new()),
        Field::Bad => return None,
        Field::Got(v) => v.value,
    };
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if !ok(item.str()) {
            let why = format!("{:?} is not a {word} id", item.str());
            c.error("value.form", Some(item.span), why)
                .label("not an id");
            return None;
        }
        if out.contains(&item.value.text) {
            let why = format!("{:?} is listed twice in `{key}`", item.str());
            c.error("value.twice", Some(item.span), why).label("again");
            return None;
        }
        out.push(item.value.text);
    }
    Some(out)
}

/// Exactly one of `instructions` and `instructions_file`; the file is
/// relative to `top` (the repository or KB root holding the definition)
/// and stays inside it.
fn instructions(c: &mut Check<'_>, top: &Path) -> Option<Instructions> {
    let inline = c.text("instructions");
    let file = c.text("instructions_file");
    match (inline, file) {
        (Field::Bad, _) | (_, Field::Bad) => None,
        (Field::Got(text), Field::Absent) => Some(Instructions::Inline(text.value.text)),
        (Field::Absent, Field::Got(path)) => instructions_file(c, top, &path),
        (Field::Got(_), Field::Got(path)) => {
            let why = "write `instructions` or `instructions_file`, not both".into();
            c.error("value.conflict", Some(path.span), why)
                .label("both");
            None
        }
        (Field::Absent, Field::Absent) => {
            let add = "instructions = \"\"\"\n…\n\"\"\"";
            c.missing("instructions", "every skill and agent profile", add);
            None
        }
    }
}

fn instructions_file(c: &mut Check<'_>, top: &Path, path: &Spanned<Text>) -> Option<Instructions> {
    match places::resolve(top, path.str()) {
        Ok(_) => Some(Instructions::File(path.value.text.clone())),
        Err(why) => {
            let why = format!("instructions_file: {why}");
            c.error("value.form", Some(path.span), why)
                .label("not a file inside this repository");
            None
        }
    }
}

fn packet(c: &mut Check<'_>) -> Option<bool> {
    match c.text("packet") {
        Field::Absent => Some(false),
        Field::Bad => None,
        Field::Got(p) => match p.str() {
            "none" => Some(false),
            "required" => Some(true),
            other => {
                let why = format!("unknown packet {other:?}");
                c.unknown_value(p.span, why, "not a packet rule", other, &PACKETS, '"');
                None
            }
        },
    }
}

fn optional(c: &mut Check<'_>, key: &str) -> Option<Option<String>> {
    match c.text(key) {
        Field::Absent => Some(None),
        Field::Bad => None,
        Field::Got(v) => Some(Some(v.value.text)),
    }
}

/// Parse and check one definition of `kind` from `text` (shown as
/// `shown`), held under `top`.
pub fn check(
    kind: DefKind,
    text: &str,
    stem: &str,
    shown: &str,
    top: &Path,
) -> Result<Def, Report> {
    let mut c = Check::read(shown, text, kind.what());
    let keys = keys(kind);
    c.unknown_keys(keys, &AGENT_KEYS, kind.not_a_key());
    schema(&mut c, kind.schema());
    let id = id(&mut c, kind, stem);
    let description = description(&mut c);
    let wants = match kind {
        DefKind::Tool => Some(Vec::new()),
        DefKind::Skill | DefKind::Agent => needs(&mut c),
    };
    let (tools, skills, plan, packet, model, instructions) = match kind {
        DefKind::Tool => (
            Some(Vec::new()),
            Some(Vec::new()),
            Some(None),
            Some(false),
            Some(None),
            Some(Instructions::None),
        ),
        DefKind::Skill => {
            let tools = ids(&mut c, "tools", DefKind::Tool);
            let plan = optional(&mut c, "plan");
            (
                tools,
                Some(Vec::new()),
                plan,
                Some(false),
                Some(None),
                instructions(&mut c, top),
            )
        }
        DefKind::Agent => {
            let skills = ids(&mut c, "skills", DefKind::Skill);
            let tools = ids(&mut c, "tools", DefKind::Tool);
            let packet = packet(&mut c);
            let model = optional(&mut c, "model");
            (
                tools,
                skills,
                Some(None),
                packet,
                model,
                instructions(&mut c, top),
            )
        }
    };
    let built = (|| {
        Some(Def {
            kind,
            id: id?,
            description: description?,
            tools: tools?,
            skills: skills?,
            plan: plan?,
            packet: packet?,
            model: model?,
            needs: wants?,
            instructions: instructions?,
            shown: shown.to_owned(),
            top: top.to_path_buf(),
        })
    })();
    c.finish(built)
}
