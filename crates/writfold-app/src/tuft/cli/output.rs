use super::repos::Prepared;
use crate::trial::Result;
use crate::tuft::Executed;
use tuft::emit::Format;

pub(super) fn resolved(
    executed: &mut Executed,
    printed: &mut String,
    prepared: &Prepared,
) -> Result<()> {
    if !prepared.echo {
        return Ok(());
    }
    let commit = &executed.plan.find.commit;
    let requested = &prepared.requested;
    let label = format!(" at={}→{commit}", serde_json::to_string(requested)?);
    match executed.emitted.format {
        Format::Compact => {
            let rows = printed
                .strip_suffix(&executed.emitted.trailer)
                .ok_or("missing compact trailer")?;
            *printed = format!("{rows}{}{label}", executed.emitted.trailer);
        }
        Format::Verbose => {
            printed.insert_str(0, &format!("at={requested:?}→{commit}\n"));
        }
        Format::Json => {
            let body = printed.strip_suffix('}').ok_or("missing JSON object")?;
            *printed = format!(
                "{body},\"at\":{}}}",
                serde_json::to_string(&Resolution { requested, commit })?
            );
        }
    }
    executed.emitted.trailer.push_str(&label);
    Ok(())
}

#[derive(serde::Serialize)]
struct Resolution<'a> {
    requested: &'a str,
    commit: &'a str,
}

#[derive(serde::Serialize)]
struct Overall<'a> {
    coverage: &'a str,
    repositories: usize,
    examined_files: usize,
    total_files: usize,
    hits: usize,
    shown: usize,
    omitted: usize,
}

pub(super) struct Part {
    pub name: String,
    pub printed: String,
    pub trailer: String,
}

fn number(trailer: &str, key: &str) -> usize {
    trailer
        .split_whitespace()
        .find_map(|v| v.strip_prefix(key))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub(super) fn combine(parts: Vec<Part>, format: Format) -> Result<(String, String)> {
    if parts.len() == 1 {
        let part = parts.into_iter().next().ok_or("no repository result")?;
        return Ok((part.printed, part.trailer));
    }
    let (mut examined, mut total, mut hits, mut shown, mut omitted) = (0, 0, 0, 0, 0);
    let mut complete = true;
    for part in &parts {
        let trailer = &part.trailer;
        complete &= trailer.split_whitespace().any(|p| p == "coverage=complete");
        if let Some((a, b)) = trailer
            .split_whitespace()
            .find_map(|p| p.strip_prefix("files="))
            .and_then(|p| p.split_once('/'))
        {
            examined += a.parse::<usize>()?;
            total += b.parse::<usize>()?;
        }
        hits += number(trailer, "hits=");
        shown += number(trailer, "shown=");
        omitted += number(trailer, "omitted=");
    }
    let coverage = if complete { "complete" } else { "partial" };
    let count = parts.len();
    let trailer = format!(
        "# overall coverage={coverage} repos={count} files={examined}/{total} hits={hits} shown={shown} omitted={omitted}"
    );
    let printed = if format == Format::Json {
        let overall = Overall {
            coverage,
            repositories: count,
            examined_files: examined,
            total_files: total,
            hits,
            shown,
            omitted,
        };
        let mut out = format!(
            "{{\"tuft\":{},\"repositories\":[",
            serde_json::to_string(tuft::VERSION)?
        );
        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"repo\":{},\"result\":{}}}",
                serde_json::to_string(&part.name)?,
                part.printed
            ));
        }
        out.push_str(&format!(
            "],\"overall\":{}}}",
            serde_json::to_string(&overall)?
        ));
        out
    } else {
        let mut out = Vec::new();
        for part in parts {
            out.push(format!(
                "# repo={}\n{}",
                serde_json::to_string(&part.name)?,
                part.printed
            ));
        }
        out.push(trailer.clone());
        out.join("\n")
    };
    Ok((printed, trailer))
}
