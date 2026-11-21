//! Every form the `kb help` overview lists is accepted by the parser
//! (#55): each verb it names is a clap subcommand with its own help, and
//! each `--flag` it shows on a verb's lines parses on that verb and is
//! shown in that verb's `--help`.
use writfold_app::kb::{USAGE, cli};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// (verb, the text of its overview entry with continuation lines).
fn entries() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in USAGE.lines() {
        match line.strip_prefix("  ").filter(|l| !l.starts_with(' ')) {
            Some(text) => {
                let verb = text.split([' ', '|']).next().unwrap_or_default();
                out.push((verb.to_owned(), text.to_owned()));
            }
            None if line.starts_with("   ") => {
                if let Some((_, text)) = out.last_mut() {
                    text.push(' ');
                    text.push_str(line.trim());
                }
            }
            None => {}
        }
    }
    out
}

fn flags(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in text.match_indices("--") {
        let name: String = text[at + 2..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .collect();
        if !name.is_empty() && !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

#[test]
fn the_overview_and_the_parser_name_the_same_verbs() -> Result {
    let listed: Vec<String> = entries().into_iter().map(|(v, _)| v).collect();
    for verb in cli::VERBS {
        assert!(listed.iter().any(|v| v == verb), "{verb} not in kb help");
    }
    for verb in &listed {
        assert!(
            cli::VERBS.contains(&verb.as_str()) || verb == "hook",
            "kb help lists {verb}, which the parser does not know"
        );
    }
    Ok(())
}

#[test]
fn every_flag_the_overview_shows_parses_on_its_verb() -> Result {
    let mut checked = 0;
    for (verb, text) in entries().into_iter().filter(|(v, _)| v != "hook") {
        let mut command = cli::command();
        command.build();
        let help = (command.find_subcommand_mut(&verb))
            .ok_or(format!("no subcommand {verb}"))?
            .render_help()
            .to_string();
        for flag in flags(&text) {
            let mut line = vec!["kb".to_owned(), verb.clone(), format!("--{flag}")];
            if cli::takes_value(&verb, &flag) {
                line.push("x".to_owned());
            }
            let parsed = cli::command().try_get_matches_from(&line);
            assert!(parsed.is_ok(), "{line:?}: {:?}", parsed.err());
            assert!(
                help.contains(&format!("--{flag}")),
                "kb {verb} --help does not show --{flag}"
            );
            checked += 1;
        }
    }
    assert!(checked > 80, "only {checked} forms");
    Ok(())
}
