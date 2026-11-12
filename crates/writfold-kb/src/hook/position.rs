//! The session-start "plan position": the current run's current step and
//! what it still owes, cut down from `plan status` (the verb's own rows,
//! so the hook says nothing `plan status` would not).
//!
//! Kept: the `[current]` step with its `read:` and `next:` lines, and each
//! done step shown `!stale` with its `why:` line. Dropped: the later steps,
//! the `done:` lines, notes and the trailer. A plan whose every step is
//! closed is one `plan complete` line; a run without a plan, nothing.

/// One answer row: its first line and its indented lines.
fn rows(text: &str) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = Vec::new();
    for line in text.lines() {
        if line.starts_with("  ") {
            if let Some(row) = out.last_mut() {
                row.push(line);
            }
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            out.push(vec![line]);
        }
    }
    out
}

/// The position block from a `plan status` answer; `None` without steps.
/// `header` opens the block (it names where the lines came from).
pub fn block(status: &str, run: &str, header: &str) -> Option<String> {
    let steps: Vec<Vec<&str>> = rows(status)
        .into_iter()
        .filter(|r| r[0].starts_with("step "))
        .collect();
    if steps.is_empty() {
        return None;
    }
    let total = steps.len();
    let current = steps.iter().position(|r| r[0].contains(" [current] "));
    let mut text = match current {
        Some(i) => format!("\n{header} for run {run}: step {}/{total}\n", i + 1),
        None => {
            return Some(format!(
                "\n{header} for run {run}: complete, every step is done or skipped ({total} steps)\n"
            ));
        }
    };
    let wanted = |row: &&Vec<&str>| row[0].contains(" [current] ") || row[0].ends_with(" !stale");
    for row in steps.iter().filter(wanted) {
        text.push_str(row[0]);
        text.push('\n');
        for more in &row[1..] {
            if !more.trim_start().starts_with("done:") {
                text.push_str(more);
                text.push('\n');
            }
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::block;

    const STATUS: &str = "step c#1/research [done] backfilled [satisfied] @a:1\n  done: recorded x\n\
step c#1/implement [done] unit [stale] @b:1 !stale\n  why: no longer holds: unit [stale]\n  done: recorded y\n\
step c#1/review [current] owes: review [open] @c:1\n  next: writfold kb gate attest review --statement TEXT --by WHO\n\
step c#1/reflect [later] gates: reflected @c:1\n# note: plan c\n# coverage=complete\n";

    #[test]
    fn current_and_stale_steps_only() {
        let found = block(STATUS, "run-1", "Plan position").unwrap_or_default();
        assert_eq!(
            found,
            "\nPlan position for run run-1: step 3/4\n\
step c#1/implement [done] unit [stale] @b:1 !stale\n  why: no longer holds: unit [stale]\n\
step c#1/review [current] owes: review [open] @c:1\n  next: writfold kb gate attest review --statement TEXT --by WHO\n"
        );
    }

    #[test]
    fn complete_and_absent() {
        let done = "step c#1/a [done] no gates @a:1\n  done: recorded x\n# trailer\n";
        assert_eq!(
            block(done, "run-1", "Plan").as_deref(),
            Some("\nPlan for run run-1: complete, every step is done or skipped (1 steps)\n")
        );
        assert_eq!(
            block(
                "# note: run run-1 has no plan\n# trailer\n",
                "run-1",
                "Plan"
            ),
            None
        );
    }
}
