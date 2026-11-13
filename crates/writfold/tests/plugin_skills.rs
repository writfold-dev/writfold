//! The plugin's skills are well formed: each `SKILL.md` opens with front
//! matter of plain `key: value` lines that Claude Code reads (a key it does
//! not know is ignored silently, so a misspelled key would be lost); its
//! `name` is its directory; `allowed-tools` pre-approves only `writfold kb`
//! commands; and every injected command (`` !`…` ``) is covered by one of
//! them, since an injected command that is not allowed aborts the skill.
//! The keys were checked against https://code.claude.com/docs/en/skills on
//! 13 November 2026.
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

/// The front matter keys Claude Code documents for a skill.
const KEYS: [&str; 18] = [
    "name",
    "description",
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "allowed-tools",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
    "license",
];

/// The front matter as key and value, and the body.
fn front_matter(text: &str) -> Result<(BTreeMap<String, String>, String)> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or("the first line must be ---")?;
    let (head, body) = rest.split_once("\n---\n").ok_or("no closing ---")?;
    let mut map = BTreeMap::new();
    for line in head.lines() {
        let (key, value) = line
            .split_once(": ")
            .ok_or_else(|| format!("not a key: value line: {line}"))?;
        let value = match value.strip_prefix('"') {
            Some(quoted) => quoted.strip_suffix('"').ok_or("an open quote")?.to_owned(),
            None => {
                // A plain YAML scalar may not hold ": " or " #", nor
                // start with an indicator character.
                if value.contains(": ") || value.contains(" #") {
                    return Err(format!("{key}: quote this value").into());
                }
                if value.starts_with(['[', '{', '*', '&', '!', '|', '>', '\'', '%', '@', '`']) {
                    return Err(format!("{key}: quote this value").into());
                }
                value.to_owned()
            }
        };
        if !KEYS.contains(&key) {
            return Err(format!("{key} is not a skill front matter key").into());
        }
        if map.insert(key.to_owned(), value).is_some() {
            return Err(format!("{key} is repeated").into());
        }
    }
    Ok((map, body.to_owned()))
}

/// `Bash(writfold kb … *)` rules, split from a space-separated list.
fn rules(allowed: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = allowed.trim();
    while !rest.is_empty() {
        let end = rest.find(')').ok_or("an unclosed rule")?;
        let rule = &rest[..=end];
        let inner = rule
            .strip_prefix("Bash(")
            .and_then(|r| r.strip_suffix(')'))
            .ok_or_else(|| format!("{rule} is not a Bash rule"))?;
        if !inner.starts_with("writfold kb ") || !inner.ends_with(" *") {
            return Err(format!("{rule} is not a writfold kb prefix rule").into());
        }
        if inner.contains(['&', '|', ';', '`', '$']) {
            return Err(format!("{rule} holds a shell operator").into());
        }
        out.push(inner.trim_end_matches(" *").to_owned());
        rest = rest[end + 1..].trim_start();
    }
    Ok(out)
}

/// The commands of `` !`…` `` lines, split at `||` (the fallback `true`
/// is a shell builtin and needs no rule).
fn injected(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let mut rest = line;
        while let Some(at) = rest.find("!`") {
            let starts = at == 0 || rest[..at].ends_with(char::is_whitespace);
            let after = &rest[at + 2..];
            let Some(end) = after.find('`') else { break };
            if starts {
                for part in after[..end].split("||") {
                    out.push(part.trim().to_owned());
                }
            }
            rest = &after[end + 1..];
        }
    }
    out
}

#[test]
fn plugin_skills_are_well_formed() -> Result {
    let skills = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/writfold/skills");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&skills)? {
        let dir = entry?.path();
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("a directory name")?
            .to_owned();
        let text = std::fs::read_to_string(dir.join("SKILL.md"))?;
        let (map, body) = front_matter(&text).map_err(|e| format!("{name}: {e}"))?;
        assert_eq!(map.get("name"), Some(&name), "{name}");
        let description = map.get("description").ok_or("a description")?;
        assert!((40..=1536).contains(&description.chars().count()), "{name}");
        let allowed = rules(map.get("allowed-tools").map_or("", String::as_str))
            .map_err(|e| format!("{name}: {e}"))?;
        for command in injected(&body) {
            if command == "true" {
                continue;
            }
            assert!(
                allowed
                    .iter()
                    .any(|p| command == *p || command.starts_with(&format!("{p} "))),
                "{name}: {command} is injected but not pre-approved"
            );
        }
        names.push(name);
    }
    names.sort();
    assert_eq!(names, ["evaluate", "kb"]);
    Ok(())
}

#[test]
fn the_evaluate_skill_runs_forked_and_splices_reflect() -> Result {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/writfold/skills/evaluate/SKILL.md");
    let (map, body) = front_matter(&std::fs::read_to_string(file)?)?;
    assert_eq!(map.get("context").map(String::as_str), Some("fork"));
    assert_eq!(map.get("background").map(String::as_str), Some("false"));
    assert_eq!(
        injected(&body),
        ["writfold kb plan use reflect", "true"],
        "the documented injection line"
    );
    // What `run.reflected` looks for, and the arguments slot.
    for needle in [
        "reflection:",
        "no proposal:",
        "writfold kb propose",
        "$ARGUMENTS",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
    Ok(())
}

#[test]
fn front_matter_refusals() {
    for bad in [
        "name: x\n",
        "---\nname x\n---\n",
        "---\nname: x\nname: y\n---\n",
        "---\nallowed_tools: Bash(writfold kb gate *)\n---\n",
        "---\ndescription: a: b\n---\n",
        "---\nargument-hint: [x]\n---\n",
    ] {
        assert!(front_matter(bad).is_err(), "{bad}");
    }
    for bad in [
        "Bash(git commit *)",
        "Bash(writfold kb gate waive)",
        "Read",
        "Bash(writfold kb x && rm *)",
    ] {
        assert!(rules(bad).is_err(), "{bad}");
    }
}
