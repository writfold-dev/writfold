//! Why `gates.met` is not met by an empty set of owed gates, and the way
//! past. A delivery a plan step owes (`deliver:<mode>`) is never counted
//! here: it is judged at its step (`step.met`), and `gates.met` is used on
//! `git commit`, where counting a PR comment would refuse the commit the
//! comment is about.
use crate::gate::subject::Subject;

/// The owed text when no gate is owed here; `planned` when the run has a
/// plan, `deliveries` the deliveries it owes.
pub fn text(
    subject: &Result<Option<Subject>, String>,
    planned: bool,
    deliveries: &[(String, String)],
) -> String {
    if planned && !deliveries.is_empty() {
        return "no gate is owed here: the run's plan names only deliveries; gates.met is not met by an empty set; next: require step.met for the delivery step, name a gate in a plan step, or leave this repository out of the rule with repos".into();
    }
    let why = match (planned, subject) {
        (true, _) => "the run's plan names none".to_owned(),
        (false, Ok(Some(s))) => format!("none is defined for {}", s.name),
        (false, _) => "none is defined here".to_owned(),
    };
    let next = match planned {
        true => "name a gate in a plan step",
        false => "define .writfold/gates/<id>.toml",
    };
    format!(
        "no gate is owed here: {why}; gates.met is not met by an empty set; next: {next}, or leave this repository out of the rule with repos"
    )
}
