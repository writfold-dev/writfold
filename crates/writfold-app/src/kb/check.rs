//! `writfold kb guard check`: parse its flags, read `--from-event FILE|-`,
//! and hand both to `writfold_kb::guard::check` (#183). Its exit code is
//! the decision: 0 allow, 2 ask, 3 deny, 1 usage.
use super::Outcome;
use std::io::Read;
use std::path::PathBuf;

use writfold_kb::guard::check::{Args, Checked, Posture, run, usage, usage_under};
use writfold_kb::hook::HookEnv;

/// The most a harness event may hold.
const INPUT_LIMIT: u64 = 8 << 20;

pub const HELP: &str = "usage: writfold kb guard check --kind shell --command LINE [--tool NAME]
       writfold kb guard check --kind file.write|file.read --path FILE [--tool NAME]
       writfold kb guard check --kind skill|spawn --name NAME [--tool NAME]
       writfold kb guard check --kind tool --tool NAME
       writfold kb guard check --from-event FILE|- [--harness ID]
       writfold kb guard check --event stop --session ID [--stop-active] [--message TEXT]
  common: [--kb DIR] [--cwd DIR] [--posture open|ask|deny] [--session ID] [--actor ID] [--actor-type T] [--json]
The guard site's decision for one event, exactly as kb hook pre-tool-use
decides it: the delivery guard, then the rules. Writes the guard log entry
(via: check); runs nothing. Exit 0 allow, 2 ask, 3 deny, 1 usage (3 under
a posture other than open).
--event stop runs the stop rules as kb hook stop does: exit 3 when a
blocking rule ends the turn (--stop-active: this stop follows a block).
A harness's own event is read through its binding: claude-code (built in)
or .writfold/harness/<id>.toml.
";

fn slot<'a>(args: &'a mut Args, flag: &str) -> Option<&'a mut Option<String>> {
    Some(match flag {
        "--kind" => &mut args.kind,
        "--command" => &mut args.command,
        "--path" => &mut args.path,
        "--name" => &mut args.name,
        "--tool" => &mut args.tool,
        "--cwd" => &mut args.cwd,
        "--session" => &mut args.session,
        "--actor" => &mut args.actor,
        "--actor-type" => &mut args.actor_type,
        "--from-event" => &mut args.from_event,
        "--harness" => &mut args.harness,
        "--posture" => &mut args.posture,
        "--message" => &mut args.message,
        _ => return None,
    })
}

fn parse(words: &[String]) -> Result<Args, String> {
    let mut args = Args::default();
    let mut rest = words.iter();
    while let Some(flag) = rest.next() {
        if flag == "--json" || flag == "--stop-active" {
            args.json |= flag == "--json";
            args.stop_active |= flag == "--stop-active";
            continue;
        }
        if flag == "--event" {
            match rest.next().map(String::as_str) {
                Some("stop") => args.stop = true,
                Some("pre-tool") => {}
                other => return Err(format!("--event takes stop or pre-tool, not {other:?}")),
            }
            continue;
        }
        let Some(slot) = slot(&mut args, flag) else {
            return Err(format!("unknown argument {flag:?}"));
        };
        if slot.is_some() {
            return Err(format!("{flag} given twice"));
        }
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        *slot = Some(value.clone());
    }
    if args.from_event.is_some() && args.kind.is_some() {
        return Err("--from-event and --kind describe the event twice; give one".into());
    }
    if args.harness.is_some() && args.from_event.is_none() {
        return Err("--harness reads --from-event; give the event".into());
    }
    Ok(args)
}

fn read_event(source: &str) -> Result<String, String> {
    let mut text = String::new();
    let read = match source {
        "-" => std::io::stdin()
            .lock()
            .take(INPUT_LIMIT)
            .read_to_string(&mut text),
        file => {
            std::fs::File::open(file).and_then(|f| f.take(INPUT_LIMIT).read_to_string(&mut text))
        }
    };
    read.map(|_| text)
        .map_err(|e| format!("cannot read the event {source}: {e}"))
}

fn outcome(c: Checked) -> Outcome {
    Outcome {
        stdout: c.stdout,
        stderr: c.stderr,
        code: c.exit,
    }
}

/// `writfold kb guard check <words>` in `cwd`.
pub fn main(words: &[String], cwd: PathBuf) -> Outcome {
    if words.iter().any(|w| w == "--help" || w == "-h") {
        return Outcome {
            stdout: HELP.into(),
            stderr: String::new(),
            code: 0,
        };
    }
    let (kb, words) = match words.iter().position(|w| w == "--kb") {
        Some(at) => match words.get(at + 1) {
            Some(dir) => {
                let rest: Vec<String> = words[..at]
                    .iter()
                    .chain(&words[at + 2..])
                    .cloned()
                    .collect();
                (Some(PathBuf::from(dir)), rest)
            }
            None => return outcome(usage("--kb needs a value")),
        },
        None => (None, words.to_vec()),
    };
    let closed = words
        .windows(2)
        .any(|w| w[0] == "--posture" && w[1] != "open");
    let refuse = |why: &str| match closed {
        true => outcome(usage_under(Posture::Deny, why)),
        false => outcome(usage(why)),
    };
    let mut args = match parse(&words) {
        Ok(args) => args,
        Err(why) => return refuse(&why),
    };
    if let Some(source) = args.from_event.take() {
        match read_event(&source) {
            Ok(text) => args.from_event = Some(text),
            Err(why) => return refuse(&why),
        }
    }
    let env = HookEnv {
        kb: kb.or_else(|| {
            std::env::var_os(super::hook::KB_OPTION)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        }),
        cwd,
        now: None,
        home: std::env::var_os("HOME").map(PathBuf::from),
    };
    outcome(run(&args, &env))
}
