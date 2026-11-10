//! Argument handling for `writfold trial evidence ...`.
use super::super::Result;
use super::{Query, label, list, show};
use std::path::Path;

/// `writfold trial evidence ...`: `args` are the words after `evidence`.
pub fn main(args: &[String], options: &[(String, String)], state: &Path) -> Result<String> {
    let option = |name: &str| {
        options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    };
    match args {
        [cmd] if cmd == "list" => {
            let limit = option("limit").map_or(Ok(100), str::parse)?;
            let query = Query {
                path: option("path"),
                commit: option("commit"),
                limit,
            };
            let (rows, trailer) = list(state, &query)?;
            let mut view: String = rows.iter().map(|r| format!("{r}\n")).collect();
            match option("out") {
                Some(out) => {
                    crate::tuft::cli::write_new(Path::new(out), &view)?;
                    Ok(trailer)
                }
                None => {
                    view.push_str(&trailer);
                    Ok(view)
                }
            }
        }
        [cmd, which] if cmd == "show" => {
            let context = option("context-lines").map_or(Ok(0), str::parse)?;
            Ok(show(state, which, context, option("repo").map(Path::new))?.text)
        }
        [cmd] if cmd == "labels" => label::render(state),
        _ => Err(
            "usage: writfold trial evidence list [--path P] [--commit C] [--limit N] [--out FILE] \
| show BINDING [--context-lines N --repo DIR] | labels"
                .into(),
        ),
    }
}
