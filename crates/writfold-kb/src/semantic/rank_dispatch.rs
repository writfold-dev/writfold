//! Semantic-only dispatch, keeping ordinary verbs and hooks on their old path.
use crate::Verb;
use crate::emit::Answer;
use crate::error::Result;
use crate::session::Session;
use crate::verbs;

pub(crate) fn answer(
    session: &Session<'_>,
    verb: &Verb,
) -> Option<Result<(Answer, &'static str, Vec<String>)>> {
    let answered = |result: Result<Answer>, name: &'static str, args: Vec<String>| {
        Some(result.map(|answer| (answer, name, args)))
    };
    match verb {
        Verb::SemanticFind {
            text,
            filter,
            signatures,
            ranking,
        } => {
            let mut args = vec![
                text.clone(),
                format!("signatures={signatures}"),
                format!("semantic={}", ranking.name()),
            ];
            args.extend(filter.kinds.iter().map(|k| format!("kind={k}")));
            args.extend(filter.project.iter().map(|p| format!("project={p}")));
            answered(
                super::find(session, text, filter, *signatures, *ranking),
                "find",
                args,
            )
        }
        Verb::SemanticEval { file, ranking } => {
            let mut args: Vec<_> = file.iter().map(|f| f.display().to_string()).collect();
            args.push(format!("semantic={}", ranking.name()));
            answered(
                verbs::eval::eval_with(session, file.as_deref(), |text| {
                    super::find(
                        session,
                        text,
                        &verbs::find::Filter::default(),
                        false,
                        *ranking,
                    )
                }),
                "find-eval",
                args,
            )
        }
        Verb::SemanticRebuild => answered(
            rebuild(session),
            "index-rebuild",
            vec!["semantic=true".into()],
        ),
        _ => None,
    }
}

fn rebuild(session: &Session<'_>) -> Result<Answer> {
    let embedded = super::rebuild(session)?;
    let mut answer = verbs::index::status(session)?;
    answer.counts.push(("semantic-embedded", embedded));
    answer.notes.push(format!("semantic index prepared; {embedded} chunks newly embedded; similarity pointers are not evidence"));
    Ok(answer)
}
