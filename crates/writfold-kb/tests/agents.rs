//! Tools, skills and agent profiles under `.writfold/` (#144): `kb lint`
//! reports each fault at its position, and `agent inspect` prints the
//! resolved closure and its digest (exit 0) or the first fault (exit 3).
//! The digest ignores comments and descriptions and follows instruction
//! bytes and skill order. A KB without these directories is unchanged.
mod support;

use support::{TestResult, World, world};
use writfold_kb::Verb;

const TOOL: &str =
    "schema = \"writfold-tool/1\"\nid = \"repo.read\"\ndescription = \"Read files.\"\n";
const SKILL_A: &str = "schema = \"writfold-skill/1\"\nid = \"verify-claims\"\ntools = [\"repo.read\"]\ninstructions = \"\"\"\nCheck each claim against the diff.\n\"\"\"\n";
const SKILL_B: &str = "schema = \"writfold-skill/1\"\nid = \"summarise\"\ninstructions_file = \"docs/summarise.md\"\n";
const AGENT: &str = "schema = \"writfold-agent-profile/1\"\nid = \"reviewer\"\ndescription = \"Reviews claims.\"\nskills = [\"verify-claims\", \"summarise\"]\ntools = [\"repo.read\"]\npacket = \"required\"\nmodel = \"reviewer\"\ninstructions = \"\"\"\nReview only the supplied change.\n\"\"\"\n";

fn put(w: &World, rel: &str, text: &str) -> TestResult {
    let path = w.kb.join(rel);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn setup(w: &World) -> TestResult {
    put(w, ".writfold/tools/repo.read.toml", TOOL)?;
    put(w, ".writfold/skills/verify-claims.toml", SKILL_A)?;
    put(w, ".writfold/skills/summarise.toml", SKILL_B)?;
    put(w, "docs/summarise.md", "Summarise in three lines.\n")?;
    put(w, ".writfold/agents/reviewer.toml", AGENT)
}

fn inspect(w: &World) -> TestResult<(serde_json::Value, u8)> {
    let mut request = w.request(Verb::Agent(writfold_kb::agents::AgentVerb::Inspect(
        writfold_kb::agents::Inspect {
            id: "reviewer".into(),
            repo: None,
        },
    )));
    request.json = true;
    let out = writfold_kb::run(&request)?;
    Ok((serde_json::from_str(&out.text)?, out.exit))
}

fn digest(w: &World) -> TestResult<String> {
    let (body, exit) = inspect(w)?;
    assert_eq!(exit, 0, "{body}");
    Ok(body["trailer"]["digest"]
        .as_str()
        .ok_or("digest")?
        .to_owned())
}

fn lint(w: &World) -> TestResult<(Vec<String>, u8)> {
    lint_in(w, None)
}

fn lint_in(w: &World, repo: Option<std::path::PathBuf>) -> TestResult<(Vec<String>, u8)> {
    let mut request = w.request(Verb::Lint {
        paths: Vec::new(),
        repo,
        deny_warnings: false,
        records: false,
    });
    request.json = true;
    let out = writfold_kb::run(&request)?;
    let body: serde_json::Value = serde_json::from_str(&out.text)?;
    let found = (body["diagnostics"].as_array().ok_or("diagnostics")?.iter())
        .map(|d| {
            format!(
                "{} {}:{}",
                d["code"].as_str().unwrap_or(""),
                d["file"].as_str().unwrap_or(""),
                d["line"]
            )
        })
        .collect();
    Ok((found, out.exit))
}

#[test]
fn a_kb_without_agents_lints_as_before() -> TestResult {
    let w = world(&[])?;
    let (found, exit) = lint(&w)?;
    assert_eq!((found.len(), exit), (0, 0), "{found:?}");
    Ok(())
}

#[test]
fn inspect_admits_a_whole_closure_and_its_digest_tracks_behaviour() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    let (found, exit) = lint(&w)?;
    assert_eq!((found.len(), exit), (0, 0), "{found:?}");
    let (body, _) = inspect(&w)?;
    let kinds: Vec<&str> = (body["rows"].as_array().ok_or("rows")?.iter())
        .filter_map(|r| r["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["agent", "skill", "skill", "tool", "tool"]);
    let first = digest(&w)?;
    // A comment and a description are not behaviour.
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &format!("# note\n{}", AGENT.replace("Reviews claims.", "Reviews.")),
    )?;
    assert_eq!(digest(&w)?, first);
    // One instruction byte is.
    put(&w, "docs/summarise.md", "Summarise in two lines.\n")?;
    let second = digest(&w)?;
    assert_ne!(second, first);
    // So is the order of skills.
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace(
            "[\"verify-claims\", \"summarise\"]",
            "[\"summarise\", \"verify-claims\"]",
        ),
    )?;
    assert_ne!(digest(&w)?, second);
    Ok(())
}

#[test]
fn inspect_refuses_the_first_fault_and_lint_reports_each() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    // An ungranted tool: the skill needs repo.read, the profile grants none.
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("tools = [\"repo.read\"]\n", ""),
    )?;
    let (body, exit) = inspect(&w)?;
    assert_eq!(exit, 3, "{body}");
    let why = body["trailer"]["refused"].as_str().unwrap_or("");
    assert!(why.contains("repo.read"), "{why}");
    let (found, exit) = lint(&w)?;
    assert_eq!(exit, 2);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("value.unmeetable .writfold/agents/reviewer.toml")),
        "{found:?}"
    );
    // A missing skill and a wrong-kind reference are refused before tools.
    put(
        &w,
        ".writfold/agents/reviewer.toml",
        &AGENT.replace("\"summarise\"]", "\"nope\", \"reviewer\"]"),
    )?;
    let (body, exit) = inspect(&w)?;
    assert_eq!(exit, 3);
    assert!(
        body["trailer"]["refused"]
            .as_str()
            .unwrap_or("")
            .contains("no skill \"nope\""),
        "{body}"
    );
    let (found, _) = lint(&w)?;
    assert!(
        found.contains(&"ref.undefined .writfold/agents/reviewer.toml:4".to_owned()),
        "{found:?}"
    );
    assert!(
        found.contains(&"ref.wrong-kind .writfold/agents/reviewer.toml:4".to_owned()),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn each_file_fault_is_at_its_line() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    put(
        &w,
        ".writfold/skills/summarise.toml",
        "schema = \"writfold-skill/1\"\nid = \"summarise\"\ncolour = \"x\"\ninstructions = \"a\"\ninstructions_file = \"docs/summarise.md\"\n",
    )?;
    put(
        &w,
        ".writfold/skills/verify-claims.toml",
        "schema = \"writfold-skill/1\"\nid = \"verify-claims\"\ninstructions_file = \"../outside.md\"\n",
    )?;
    put(
        &w,
        ".writfold/tools/bad.toml",
        "schema = \"writfold-tool/1\"\nid = \"a.b.c\"\n",
    )?;
    let (found, exit) = lint(&w)?;
    assert_eq!(exit, 2);
    for want in [
        "key.unknown .writfold/skills/summarise.toml:3",
        "value.conflict .writfold/skills/summarise.toml:5",
        "value.form .writfold/skills/verify-claims.toml:3",
        "value.form .writfold/tools/bad.toml:2",
    ] {
        assert!(found.contains(&want.to_owned()), "{want} not in {found:?}");
    }
    Ok(())
}

/// Pins: a skill or profile with neither `instructions` nor
/// `instructions_file` is a `key.missing` fault at its file, and
/// `agent inspect` refuses the closure.
#[test]
fn a_definition_with_neither_instruction_key_is_a_fault() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    put(
        &w,
        ".writfold/skills/verify-claims.toml",
        "schema = \"writfold-skill/1\"\nid = \"verify-claims\"\ntools = [\"repo.read\"]\n",
    )?;
    let (found, exit) = lint(&w)?;
    assert_eq!(exit, 2, "{found:?}");
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("key.missing .writfold/skills/verify-claims.toml")),
        "{found:?}"
    );
    assert_eq!(inspect(&w)?.1, 3);
    setup(&w)?;
    let bare = AGENT.replace(
        "instructions = \"\"\"\nReview only the supplied change.\n\"\"\"\n",
        "",
    );
    assert_ne!(bare, AGENT);
    put(&w, ".writfold/agents/reviewer.toml", &bare)?;
    let (found, exit) = lint(&w)?;
    assert_eq!(exit, 2, "{found:?}");
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("key.missing .writfold/agents/reviewer.toml")),
        "{found:?}"
    );
    assert_eq!(inspect(&w)?.1, 3);
    Ok(())
}

/// Pins: an id defined in the KB root and in the repository under review
/// is `file.twice` for each kind, reported at the repository's file, and
/// `agent inspect --repo` refuses the profile that is in both.
#[test]
fn an_id_in_both_roots_is_file_twice_for_each_kind() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    let both = [
        (".writfold/tools/repo.read.toml", TOOL),
        (".writfold/skills/verify-claims.toml", SKILL_A),
        (".writfold/agents/reviewer.toml", AGENT),
    ];
    for (rel, text) in both {
        let path = w.code.join(rel);
        std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
        std::fs::write(path, text)?;
    }
    let (found, exit) = lint_in(&w, Some(w.code.clone()))?;
    assert_eq!(exit, 2, "{found:?}");
    for (rel, _) in both {
        let want = format!("file.twice demo-service:{rel}");
        assert!(
            found.iter().any(|f| f.starts_with(&want)),
            "{want} in {found:?}"
        );
    }
    let mut request = w.request(Verb::Agent(writfold_kb::agents::AgentVerb::Inspect(
        writfold_kb::agents::Inspect {
            id: "reviewer".into(),
            repo: Some(w.code.clone()),
        },
    )));
    request.json = true;
    let out = writfold_kb::run(&request)?;
    assert_eq!(out.exit, 3, "{}", out.text);
    assert!(
        out.text
            .contains("defined in both the KB root and the repository"),
        "{}",
        out.text
    );
    Ok(())
}

/// Pins: a skill's `plan` naming no template is `ref.undefined` at the
/// skill's `plan` line, and defining the template clears it.
#[test]
fn a_skill_plan_naming_no_template_is_undefined() -> TestResult {
    let w = world(&[])?;
    setup(&w)?;
    let planned = SKILL_A.replace("tools =", "plan = \"nowhere\"\ntools =");
    put(&w, ".writfold/skills/verify-claims.toml", &planned)?;
    let (found, exit) = lint(&w)?;
    assert_eq!(exit, 2, "{found:?}");
    assert!(
        found.contains(&"ref.undefined .writfold/skills/verify-claims.toml:3".to_owned()),
        "{found:?}"
    );
    put(
        &w,
        ".writfold/plans/nowhere.toml",
        "schema = \"writfold-plan/1\"\nname = \"nowhere\"\nsteps = [\"a:\"]\n",
    )?;
    let (found, exit) = lint(&w)?;
    assert_eq!((found.len(), exit), (0, 0), "{found:?}");
    Ok(())
}
