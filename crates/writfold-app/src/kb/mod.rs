//! `writfold kb`: Git-native project continuity reads (KB v1). Argument
//! parsing and exit codes only; every decision lives in `writfold-kb`.
pub mod args;
mod checkpoint;
mod cite;
pub mod cli;
mod deliver;
mod gate;
mod guard;
mod hook;
mod investigation;
mod lists;
mod observe;
mod plan;
mod proposals;
mod set;
mod shown;
mod usage;
mod verbs;
mod writes;

pub use writfold_kb::diag::Style;

pub use usage::USAGE;

/// The process outcome: text for stdout, text for stderr, exit code.
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

/// Run `writfold kb <args>` in `cwd`, diagnostics plain.
pub fn main(args: &[String], cwd: std::path::PathBuf) -> Outcome {
    let styles = Styles {
        stdout: Style::plain(),
        stderr: Style::plain(),
    };
    main_styled(args, cwd, &styles)
}

/// How diagnostics are drawn on each stream: the caller decides colour
/// and width (a terminal, `NO_COLOR`, `TERM=dumb`).
pub struct Styles {
    pub stdout: Style,
    pub stderr: Style,
}

/// Run `writfold kb <args>` in `cwd`, diagnostics drawn in `styles`
/// (`--color always|never` overrides the caller's choice).
pub fn main_styled(args: &[String], cwd: std::path::PathBuf, styles: &Styles) -> Outcome {
    if let Some((first, rest)) = args.split_first()
        && first == "hook"
    {
        return hook::main(rest, cwd);
    }
    let styles = coloured(args, styles);
    if let Some(outcome) = overview(args) {
        return outcome;
    }
    let request = match args::parse(args, cwd) {
        args::Parsed::Request(request) => request,
        args::Parsed::Help(text) => return said(text, styles.stdout.colour, 0),
        args::Parsed::Refused(error) => return said(error.render(), styles.stderr.colour, 1),
        args::Parsed::Usage(message) => {
            return Outcome {
                stdout: String::new(),
                stderr: format!(
                    "writfold kb: {message}\n(see: writfold kb help, or writfold kb <verb> --help)\n"
                ),
                code: 1,
            };
        }
    };
    let session = writfold_kb::seat::session_from_env();
    match writfold_kb::run_as(&request, session.as_deref()) {
        Ok(output) => Outcome {
            stderr: shown::stderr(&output, &styles.stderr),
            stdout: shown::stdout(&output, &styles.stdout),
            code: output.exit,
        },
        Err(error) => Outcome {
            stdout: String::new(),
            stderr: shown::error(&error, &styles.stderr),
            code: error.exit_code(),
        },
    }
}

/// `kb`, `kb help`, `kb --help` and `--help` before any verb: the overview.
/// `kb help <verb>` is that verb's help.
fn overview(args: &[String]) -> Option<Outcome> {
    let first = args.first().map(String::as_str);
    let verb = |v: &str| cli::VERBS.contains(&v);
    let help = |a: &String| matches!(a.as_str(), "--help" | "-h");
    let text = match (first, args.get(1)) {
        (None, _) => {
            return Some(Outcome {
                stdout: String::new(),
                stderr: format!("writfold kb: missing verb\n{USAGE}\n"),
                code: 1,
            });
        }
        (Some("help"), Some(v)) if verb(v) => {
            let mut command = cli::command();
            command.build();
            let rendered = command.find_subcommand_mut(v).map(|c| c.render_help());
            rendered.map_or_else(|| USAGE.to_owned(), |r| r.to_string())
        }
        (Some("help" | "--help" | "-h"), _) => USAGE.to_owned(),
        (Some(v), _) if !verb(v) && args.iter().any(help) => USAGE.to_owned(),
        _ => return None,
    };
    Some(Outcome {
        stdout: format!("{}\n", text.trim_end()),
        stderr: String::new(),
        code: 0,
    })
}

/// The styles with `--color always|never` applied (the last one given).
fn coloured(args: &[String], styles: &Styles) -> Styles {
    let at = args
        .iter()
        .rposition(|a| a == "--color" || a.starts_with("--color="));
    let value = at.and_then(|i| match args[i].split_once('=') {
        Some((_, v)) => Some(v.to_owned()),
        None => args.get(i + 1).cloned(),
    });
    let pick = |style: &Style| Style {
        colour: match value.as_deref() {
            Some("always") => true,
            Some("never") => false,
            _ => style.colour,
        },
        ..*style
    };
    Styles {
        stdout: pick(&styles.stdout),
        stderr: pick(&styles.stderr),
    }
}

/// Text clap rendered, coloured or plain, on stdout (exit 0) or stderr.
fn said(text: clap::builder::StyledStr, colour: bool, code: u8) -> Outcome {
    let text = match colour {
        true => text.ansi().to_string(),
        false => text.to_string(),
    };
    let (stdout, stderr) = match code {
        0 => (text, String::new()),
        _ => (String::new(), text),
    };
    Outcome {
        stdout,
        stderr,
        code,
    }
}
