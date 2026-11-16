//! The `pre-tool-use` hook over the KB's guard rules: one valid rule,
//! then the same with five rule files that do not parse. Broken files are
//! read and checked once, like the valid one, and reported as compact
//! lines: the time must stay within the first case's noise.
use super::{Result, summary};
use std::path::Path;
use std::time::Instant;
use writfold_kb::hook::{HookEnv, run};

const RULE: &str = "schema = \"writfold-guard-rule/1\"\nid = \"commit\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gates.met\"]\naction = \"deny\"\n";

fn time_hook(kb: &Path, runs: usize) -> (f64, f64, String) {
    let env = HookEnv {
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        now: None,
        home: None,
    };
    let input = serde_json::json!({"hook_event_name": "PreToolUse", "tool_name": "Bash",
        "tool_input": {"command": "git status"}, "session_id": "latency", "cwd": kb});
    let mut samples = Vec::with_capacity(runs);
    let mut stderr = String::new();
    for _ in 0..runs {
        let started = Instant::now();
        stderr = run("pre-tool-use", &input.to_string(), &env).stderr;
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let (median, p95) = summary(samples);
    (median, p95, stderr)
}

/// Time the hook with one valid rule, then with five broken ones added.
pub fn hooks(kb: &Path, runs: usize) -> Result {
    let dir = kb.join(".writfold/guards");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("commit.toml"), RULE)?;
    println!(
        "{:<40} {:>10} {:>10}",
        "hook (pre-tool-use)", "median ms", "p95 ms"
    );
    let (median, p95, _) = time_hook(kb, runs);
    println!("{:<40} {median:>10.1} {p95:>10.1}", "one valid rule");
    for n in 0..5 {
        let broken = RULE
            .replace("id = \"commit\"", &format!("id = \"broken-{n}\""))
            .replace("gates.met", "gates.mett");
        std::fs::write(dir.join(format!("broken-{n}.toml")), broken)?;
    }
    let (median, p95, stderr) = time_hook(kb, runs);
    println!(
        "{:<40} {median:>10.1} {p95:>10.1}",
        "plus five broken rule files"
    );
    print!(
        "{}",
        stderr
            .lines()
            .take(1)
            .map(|l| format!("  {l}\n"))
            .collect::<String>()
    );
    for n in 0..5 {
        std::fs::remove_file(dir.join(format!("broken-{n}.toml")))?;
    }
    std::fs::remove_file(dir.join("commit.toml"))?;
    Ok(())
}
