use super::args::Args;
use crate::trial::Result;
use std::io::Read;
use tuft::{Diagnostic, Plan};

pub(super) struct Program {
    pub source: Vec<u8>,
    pub name: String,
    pub plan: Plan,
    pub strict: bool,
}

fn quoted(value: &str) -> Result<String> {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                return Err("program argument contains an unsupported control character".into());
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(out)
}

pub(super) fn diagnostics<T>(
    result: std::result::Result<T, Vec<Diagnostic>>,
    source: &[u8],
    name: &str,
) -> Result<T> {
    result.map_err(|ds| tuft::diag::render_all(&ds, source, name).into())
}

fn grep(args: &Args) -> Result<Vec<u8>> {
    let needle = args.operand()?;
    let matcher = if args.flag("regex") {
        if needle.contains(['\n', '\r']) {
            return Err("a regex must fit on one line".into());
        }
        format!("rx\"{}\"", needle.replace('"', "\\x22"))
    } else {
        format!("text {}", quoted(needle)?)
    };
    let scope = args
        .option("in")
        .map(quoted)
        .transpose()?
        .map_or(String::new(), |s| format!(" in {s}"));
    let at = quoted(args.option("at").unwrap_or("HEAD"))?;
    let limit = args.option("limit").unwrap_or("100");
    if !limit.bytes().all(|b| b.is_ascii_digit()) || limit.is_empty() {
        return Err("--limit must be a positive decimal integer".into());
    }
    Ok(format!("@h := find {matcher}{scope} at {at} limit {limit};\n@e := retain @h around lines 0;\n@v := project @e [path, line, excerpt(max_chars: 300)] sort [path asc, line asc];\nemit @v budget_tokens 4000;\n").into_bytes())
}

pub(super) fn load(args: &Args) -> Result<Program> {
    let is_grep = args.positional.first().is_some_and(|s| s == "grep");
    let operand = args.operand()?;
    let name = if is_grep { "<grep>" } else { operand }.to_owned();
    let strict = args.flag("pinned") || (!is_grep && operand != "-");
    let mut source = if is_grep { grep(args)? } else { Vec::new() };
    if !is_grep {
        let mut reader: Box<dyn Read> = if operand == "-" {
            Box::new(std::io::stdin().lock())
        } else {
            Box::new(std::fs::File::open(operand)?)
        };
        reader
            .by_ref()
            .take((super::super::MAX_PROGRAM_BYTES + 1) as u64)
            .read_to_end(&mut source)?;
    }
    if source.len() > super::super::MAX_PROGRAM_BYTES {
        return Err("program exceeds 65536 bytes".into());
    }
    let check = if strict {
        tuft::compile
    } else {
        tuft::compile_unpinned
    };
    let mut plan = diagnostics(check(&source), &source, &name)?;
    if let Some(at) = args.option("at").filter(|_| !is_grep) {
        // The syntax and types are already checked; CLI resolution replaces only at.
        if strict && !full_commit(at) {
            return Err("--pinned requires --at to be a full lowercase commit ID".into());
        }
        plan.find.commit = at.to_owned();
    }
    Ok(Program {
        source,
        name,
        plan,
        strict,
    })
}

pub(super) fn full_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
