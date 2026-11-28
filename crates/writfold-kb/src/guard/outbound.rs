//! The delivery guard under an `unclassified` policy (#187): every simple
//! command and tool call of one call gets a class, and the proved
//! [`writfold_core::delivery::outbound::outbound`] decides. A routed or
//! listed member is judged as before; one the team's `local` list names
//! passes; a program that runs its arguments is opaque; anything else is
//! unclassified and gets the policy. `log` passes and keeps the command in
//! the guard log, which is how `local` is built before `ask` is turned on.
//! `local` is what a team says is local, not a detector of external writes.
use super::delivery_body::{self as body, Matched, Segment};
use super::event;
use crate::deliver::config::Guard;
use crate::shell;
use crate::workspace::Workspace;
use serde_json::{Map, Value, json};
use std::path::Path;
use writfold_core::delivery::outbound::{Class, Unclassified, outbound};
use writfold_core::guard::Decision;

/// The ssh options that take a value.
const SSH_VALUED: [&str; 18] = [
    "-b", "-c", "-D", "-E", "-e", "-F", "-I", "-i", "-J", "-L", "-l", "-m", "-O", "-o", "-p", "-Q",
    "-R", "-S",
];

/// Whether a simple command runs its arguments as a program.
pub fn opaque(words: &[String], text: &str) -> bool {
    let first = words.first().map(String::as_str).unwrap_or("");
    let has = |w: &str| words.iter().skip(1).any(|x| x == w);
    let shell_c = matches!(first, "bash" | "sh" | "zsh" | "dash" | "ksh") && has("-c");
    let find_exec = first == "find" && (has("-exec") || has("-execdir") || has("-ok"));
    text.contains('`')
        || text.contains("$(")
        || matches!(first, "eval" | "xargs" | "watch")
        || shell_c
        || find_exec
        || (first == "ssh" && ssh_command(&words[1..]))
}

/// `ssh` with a command: a host and at least one more word after options.
fn ssh_command(rest: &[String]) -> bool {
    let mut positional = 0;
    let mut words = rest.iter();
    while let Some(w) = words.next() {
        if SSH_VALUED.contains(&w.as_str()) {
            words.next();
        } else if !w.starts_with('-') {
            positional += 1;
        }
    }
    positional >= 2
}

/// What the delivery guard decided under a policy.
pub struct Outbound {
    pub decision: Decision,
    pub reasons: Vec<String>,
    /// The unclassified commands, for the guard log under `log`.
    pub unclassified: Vec<String>,
}

fn way_out(name: &str, policy: Unclassified) -> String {
    format!(
        "writfold delivery guard (a guard, not enforcement): `{name}` is not classified (unclassified = \"{}\"): route it in .writfold/deliveries/guard.toml, or add it to local there if what it writes stays here",
        policy.name()
    )
}

/// What a member no route or list covers is, by name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Named {
    Local,
    Opaque,
    Other,
}

type Member = (String, Option<Matched>, Named);

/// Classify every member of the call and decide.
pub fn judge(
    ws: &Workspace,
    guard: &Guard,
    policy: Unclassified,
    (input, cwd): (&Map<String, Value>, &Path),
    decide: &dyn Fn(&Workspace, &Matched) -> Option<String>,
) -> Outbound {
    let members = match event::is_shell(input) {
        true => shell_members(guard, input, cwd),
        false => tool_member(guard, input, cwd),
    };
    let mut classes = Vec::new();
    let mut out = Outbound {
        decision: Decision::Pass,
        reasons: Vec::new(),
        unclassified: Vec::new(),
    };
    for (name, matched, named) in members {
        let class = match (&matched, named) {
            (Some(m), _) => {
                let refused = decide(ws, m);
                let d = if refused.is_some() {
                    Decision::Deny
                } else {
                    Decision::Pass
                };
                out.reasons.extend(refused);
                if m.routed {
                    Class::Routed(d)
                } else {
                    Class::Listed(d)
                }
            }
            (None, Named::Local) => Class::Local,
            (None, Named::Opaque) => Class::Opaque,
            (None, Named::Other) => Class::Unclassified,
        };
        if matches!(class, Class::Opaque | Class::Unclassified) {
            if matches!(policy, Unclassified::Ask | Unclassified::Deny) {
                out.reasons.push(way_out(&name, policy));
            }
            out.unclassified.push(name);
        }
        classes.push(class);
    }
    out.decision = outbound(&classes, policy);
    out
}

/// Each simple command, named by its first three words. An opaque program
/// is never local, whatever the list says.
fn shell_members(guard: &Guard, input: &Map<String, Value>, cwd: &Path) -> Vec<Member> {
    let line = (input
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str))
    .unwrap_or("");
    body::segments(guard, line, cwd)
        .into_iter()
        .filter(|s: &Segment| !s.words.is_empty())
        .map(|s| {
            let named = if opaque(&s.words, &s.text) {
                Named::Opaque
            } else if guard.local.iter().any(|l| shell::matches(l, &s.words)) {
                Named::Local
            } else {
                Named::Other
            };
            (
                s.words
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
                s.matched,
                named,
            )
        })
        .collect()
}

fn tool_member(guard: &Guard, input: &Map<String, Value>, cwd: &Path) -> Vec<Member> {
    let tool = input
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let matched = body::matched(guard, input, cwd);
    let named = match guard.local_tools.contains(&tool) {
        true => Named::Local,
        false => Named::Other,
    };
    vec![(tool, matched, named)]
}

/// The guard log entries for unclassified commands passed under `log`.
pub fn entries(session: &str, tool: &str, commands: &[String], now: i64) -> Vec<Value> {
    commands
        .iter()
        .map(|c| {
            json!({"schema": super::log::SCHEMA, "at": crate::time::stamp(now), "via": "hook",
                "session": session, "kind": "unclassified", "tool": tool,
                "command": super::log::command(c)})
        })
        .collect()
}
