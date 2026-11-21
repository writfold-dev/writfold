//! The verbs `kb` measures before it answers (`runs --idle`, `find --eval`,
//! `find --each`), moved out of `lib.rs` for its size budget.
use crate::Verb;
use crate::emit::Answer;
use crate::error::Result;
use crate::session::Session;

/// `runs --idle` and `find --eval` (v0.6.0-dev.1), `find --each`
/// (v0.6.0-dev.6); `None` for every other verb.
pub fn measured(
    session: &Session<'_>,
    verb: &Verb,
) -> Option<Result<(Answer, &'static str, Vec<String>)>> {
    let answered =
        |r: Result<Answer>, name: &'static str, args: Vec<String>| Some(r.map(|a| (a, name, args)));
    match verb {
        Verb::RunsIdle {
            age,
            age_text,
            project,
            plan,
        } => {
            let mut args = vec![format!("idle={age_text}")];
            args.extend(project.iter().map(|p| format!("project={p}")));
            if *plan {
                args.push("plan=true".into());
            }
            answered(
                crate::verbs::runs::idle(session, (*age, age_text), project.as_deref(), *plan),
                "runs",
                args,
            )
        }
        Verb::FindEach {
            texts,
            filter,
            signatures,
        } => answered(
            crate::verbs::find_each::each(session, texts, filter, *signatures),
            "find",
            crate::verbs::find_each::args(texts, filter, *signatures),
        ),
        Verb::FindEval { file } => answered(
            crate::verbs::eval::eval(session, file.as_deref()),
            "find-eval",
            file.iter().map(|f| f.display().to_string()).collect(),
        ),
        _ => None,
    }
}
