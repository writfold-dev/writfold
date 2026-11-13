use super::args::Args;
use crate::trial::Result;
use serde_json::json;
use std::fmt;
use tuft::emit::Format;

#[derive(Debug)]
pub(super) struct InvalidJson(pub String);
impl fmt::Display for InvalidJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for InvalidJson {}

pub(super) fn check(args: &Args) -> Result<String> {
    let name = args.operand()?;
    let source = std::fs::read(name)?;
    if source.len() > super::super::MAX_PROGRAM_BYTES {
        return Err("program exceeds 65536 bytes".into());
    }
    let checked = tuft::compile(&source);
    if args.format == Format::Json {
        let diagnostics: Vec<_> = checked
            .as_ref()
            .err()
            .into_iter()
            .flatten()
            .map(|d| {
                let before = &source[..d.span.lo.min(source.len())];
                let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
                let column = before.len()
                    - before
                        .iter()
                        .rposition(|&b| b == b'\n')
                        .map_or(0, |n| n + 1)
                    + 1;
                json!({"code": d.code.as_str(), "message": d.message,
                "span": {"lo": d.span.lo, "hi": d.span.hi}, "line": line, "column": column})
            })
            .collect();
        let value = json!({"tuft": tuft::VERSION, "ok": checked.is_ok(), "file": name,
            "program": checked.as_ref().ok().map(|p| format!("sha256:{}", p.program)), "diagnostics": diagnostics}).to_string();
        return if checked.is_ok() {
            Ok(value)
        } else {
            Err(Box::new(InvalidJson(value)))
        };
    }
    let plan = super::program::diagnostics(checked, &source, name)?;
    let (matcher, pattern) = plan.find.matcher.describe();
    Ok(format!(
        "ok {} program=sha256:{}\n  find @{} {matcher} {:?} in {:?} at {} limit={}\n  retain {}\n  project @{} [{}]\n  emit budget_tokens={}",
        tuft::VERSION,
        plan.program,
        plan.find.binding,
        pattern,
        plan.find.scope,
        plan.find.commit,
        plan.find.budget.positions(),
        plan.retain.as_ref().map_or("none".to_owned(), |r| format!(
            "@{} around={:?}",
            r.binding, r.around
        )),
        plan.view.binding,
        plan.view
            .fields
            .iter()
            .map(|f| f.name())
            .collect::<Vec<_>>()
            .join(", "),
        plan.emit.budget_tokens
    ))
}
