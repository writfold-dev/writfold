//! The listing verbs built in v0.5.0-dev.10 that need more than a switch:
//! `runs --idle AGE`.
use super::args::Common;
use writfold_kb::Verb;

/// `runs --idle AGE [--project P] [--plan]`.
pub fn idle(common: &Common) -> Result<Verb, String> {
    let text = common.values.get("idle").cloned().unwrap_or_default();
    if common.switches.iter().any(|s| s == "open") {
        return Err("runs --idle lists open runs already; drop --open".into());
    }
    let age = writfold_kb::verbs::runs::idle_age(&text)
        .ok_or_else(|| format!("--idle takes an age such as 7d or 36h, not {text}"))?;
    Ok(Verb::RunsIdle {
        age,
        age_text: text,
        project: common.project.clone(),
        plan: common.switches.iter().any(|s| s == "plan"),
    })
}
