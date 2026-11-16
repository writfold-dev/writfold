//! A waiver a passing guard rule rested on: `gates.met` and `gate:<id>`
//! accept a waived gate, and the guard's answers say so (`!waived` in
//! `guard status`, a stderr line and a log entry from the hook) rather
//! than reading as an ordinary pass.
use crate::gate::status::Judged;
use writfold_core::gate::Standing;

/// One waived gate a requirement was met by.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Waived {
    pub gate: String,
    /// `<reason>, by <who> at <when>`.
    pub text: String,
    /// The statement's KB-relative path.
    pub path: String,
}

/// The waiver a judged gate stands on, when it stands as waived.
pub fn of(judged: &Judged<'_>) -> Option<Waived> {
    if judged.standing != Standing::Waived {
        return None;
    }
    let (filed, statement) = judged.statements.waived?;
    Some(Waived {
        gate: judged.definition.id.clone(),
        text: format!(
            "{}, by {} at {}",
            statement.text, statement.by, statement.recorded
        ),
        path: filed.path.clone(),
    })
}
