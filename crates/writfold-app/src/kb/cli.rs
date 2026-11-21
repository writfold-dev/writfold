//! The `writfold kb` command line as a clap command (#55), built from the
//! verbs' own flag tables so the parser, the per-verb help and the
//! did-you-mean suggestions cannot drift apart. clap only splits the line;
//! which flags apply to a verb and what they mean stays in [`super::args`]
//! and the verb modules. Tested, not proved: no decision is made here.
use super::args::Common;
use super::{investigation, observe, proposals, usage, writes};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

/// Every verb, as `kb help` lists them (`hook` is dispatched earlier).
pub const VERBS: &[&str] = &[
    "brief",
    "context",
    "why",
    "status",
    "find",
    "match",
    "impact",
    "uses",
    "timeline",
    "stale",
    "repos",
    "index",
    "check",
    "init",
    "policy",
    "lint",
    "runs",
    "delta",
    "resume",
    "guard",
    "checkpoint",
    "record",
    "cite",
    "observe",
    "checker",
    "propose",
    "proposals",
    "gate",
    "deliver",
    "plan",
];

/// Flags every verb takes.
const SHARED: [&str; 5] = ["kb", "budget-tokens", "cursor", "json", "color"];

/// The flags that are typed fields of [`Common`] rather than write values.
const FIELDS: [&str; 12] = [
    "all",
    "depth",
    "offline",
    "cached",
    "record",
    "kind",
    "project",
    "signatures",
    "direction",
    "include",
    "missing",
    "limit",
];

/// Switches among [`FIELDS`] (the rest take a value).
const FIELD_SWITCHES: [&str; 5] = ["all", "offline", "cached", "record", "missing"];

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Switch,
    Value,
    Repeat,
}

/// How `flag` is written on `verb`'s line.
fn shape(verb: &str, flag: &str) -> Shape {
    let switch = FIELD_SWITCHES.contains(&flag)
        || flag == "json"
        || flag == "again"
        || writes::SWITCHES.contains(&flag)
        || proposals::SWITCHES.contains(&flag)
        // `record|propose investigation --signatures a,b` takes a value.
        || (flag == "signatures" && !matches!(verb, "record" | "propose"));
    match () {
        () if switch => Shape::Switch,
        () if super::checkpoint::REPEATS.contains(&flag) => Shape::Repeat,
        () => Shape::Value,
    }
}

/// Every flag any verb takes, once each.
fn every_flag() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = SHARED.iter().chain(FIELDS.iter()).copied().collect();
    let tables: [&[&'static str]; 6] = [
        &writes::VALUES,
        &writes::SWITCHES,
        &observe::VALUES,
        &investigation::VALUES,
        &proposals::VALUES,
        &proposals::SWITCHES,
    ];
    for flag in tables.into_iter().flatten().chain(["again"].iter()) {
        if !all.contains(flag) {
            all.push(flag);
        }
    }
    all
}

/// The flags `verb`'s help shows: the shared ones and those it accepts.
fn shown(verb: &str, flag: &str) -> bool {
    SHARED.contains(&flag)
        || super::args::allowed(verb).contains(&flag)
        || writes::allowed(verb).contains(&flag)
}

fn flag(verb: &str, name: &'static str) -> Arg {
    let arg = Arg::new(name).long(name).hide(!shown(verb, name));
    let arg = match shape(verb, name) {
        Shape::Switch => arg.action(ArgAction::SetTrue),
        Shape::Value => arg
            .action(ArgAction::Set)
            .allow_hyphen_values(true)
            .value_name(upper(name)),
        Shape::Repeat => {
            (arg.action(ArgAction::Append).allow_hyphen_values(true)).value_name(upper(name))
        }
    };
    match name {
        "kb" => arg.help("the KB root (else $WRITFOLD_KB, else found from here)"),
        "budget-tokens" => arg.help("cut the answer to about this many tokens"),
        "cursor" => arg.help("continue a cut answer from its cursor"),
        "json" => arg.help("one JSON object instead of rows"),
        "color" => (arg.value_parser(["auto", "always", "never"]))
            .help("colour diagnostics (auto: on a terminal, unless NO_COLOR)"),
        _ => arg,
    }
}

/// `budget-tokens` as `BUDGET_TOKENS`, for the help's value names.
fn upper(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

fn verb_command(verb: &'static str) -> Command {
    let mut command = Command::new(verb)
        .about(usage::section(verb))
        .disable_help_subcommand(true)
        .arg(
            Arg::new("args")
                .num_args(0..)
                .action(ArgAction::Append)
                .value_name("ARG"),
        )
        // `--at COMMIT` is refused with its own message ([`super::args`]).
        .arg(
            Arg::new("at")
                .long("at")
                .hide(true)
                .allow_hyphen_values(true),
        );
    for name in every_flag() {
        command = command.arg(flag(verb, name));
    }
    command
}

/// The whole `writfold kb` command.
pub fn command() -> Command {
    Command::new("kb")
        .bin_name("writfold kb")
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand_required(true)
        .override_help(usage::USAGE)
        .subcommands(VERBS.iter().map(|v| verb_command(v)))
}

/// `verb`'s flags and positionals from clap's matches into [`Common`],
/// with the same messages for bad numbers as before.
pub fn common(verb: &str, m: &ArgMatches) -> Result<(Common, Vec<String>), String> {
    let mut c = Common::default();
    if m.contains_id("at") && m.get_one::<String>("at").is_some() {
        return Err("--at COMMIT is not built in v1 (planned for M4)".into());
    }
    for name in every_flag() {
        match shape(verb, name) {
            Shape::Switch if m.get_flag(name) => switch(&mut c, name),
            Shape::Switch => {}
            Shape::Value => {
                if let Some(value) = m.get_one::<String>(name) {
                    value_of(&mut c, name, value.clone())?;
                }
            }
            Shape::Repeat => {
                for value in m.get_many::<String>(name).into_iter().flatten() {
                    super::checkpoint::put(&mut c.values, name, value.clone());
                }
            }
        }
    }
    let positional = m.get_many::<String>("args").into_iter().flatten().cloned();
    Ok((c, positional.collect()))
}

fn switch(c: &mut Common, name: &str) {
    match name {
        "json" => c.json = true,
        "all" => c.all = true,
        "offline" => c.offline = true,
        "cached" => c.cached = true,
        "record" => c.record = true,
        "missing" => c.missing = true,
        "signatures" => c.signatures = true,
        other => c.switches.push(other.to_owned()),
    }
}

fn value_of(c: &mut Common, name: &str, value: String) -> Result<(), String> {
    let number = |low, high| super::args::number(&format!("--{name}"), &value, low, high);
    match name {
        "kb" => c.kb = Some(PathBuf::from(value)),
        "budget-tokens" => c.budget = Some(number(1, writfold_kb::MAX_BUDGET)?),
        "depth" => c.depth = Some(number(1, 8)?),
        "limit" => c.limit = Some(number(1, 50)?),
        "cursor" => c.cursor = Some(value),
        "color" => c.color = Some(value),
        "kind" => {
            c.kinds = (value.split(','))
                .map(|k| k.trim().to_owned())
                .filter(|k| !k.is_empty())
                .collect();
        }
        "project" => c.project = Some(value),
        "direction" => c.direction = Some(value),
        "include" => c.include = Some(value),
        other => super::checkpoint::put(&mut c.values, other, value),
    }
    Ok(())
}

/// Whether `flag` takes a value on `verb`'s line (for tests and help).
pub fn takes_value(verb: &str, flag: &str) -> bool {
    shape(verb, flag) != Shape::Switch
}
