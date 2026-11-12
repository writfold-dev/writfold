//! Retrieval receipts: what a retrieval tool (Tuft) showed during a run,
//! recorded in the run as one new Markdown file,
//! `runs/<id>/retrievals/<UTC compact time>-<6 hex>.md`, written create-new
//! through the write protocol. Front matter states the tool, the program
//! digest, the repository and commit searched, and the row counts; the body
//! holds the tool's coverage trailer and one code citation token per cited
//! span, the same tokens `cite make` prints, so `stale` checks them like any
//! other citation. The prototype reads only `receipts/` (its own numbered
//! JSON) and never this directory; Writfold indexes the file as a document.
//!
//! This module also cites spans for the adapter ([`cite_spans`]) and shows
//! a run's newest receipts for `resume` ([`rows`]).
use super::cite::{Fields, Span, code_token, seen, spans, url};
use super::{Op, Plan, one_line};
use crate::emit::{More, Row};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::parse::cite::tokens;
use crate::runs;
use crate::session::Session;
use crate::stale::repos;
use crate::time;
use crate::workspace::{GitTool, Workspace};
use sha2::{Digest, Sha256};
use std::path::Path;
use writfold_sources::frontmatter::parse_frontmatter;

/// The directory of a run that holds its retrieval receipts.
pub const DIR: &str = "retrievals";
/// Receipts `resume` shows, newest first.
const SHOWN: usize = 3;
/// Citations shown under each receipt in `resume`.
const CITES_SHOWN: usize = 5;

/// One retrieval to record.
#[derive(Clone, Debug, Default)]
pub struct Retrieval {
    /// The run; `None` takes the current-run pointer.
    pub run: Option<String>,
    /// The tool and its version, such as `tuft/0`.
    pub tool: String,
    /// The program digest, `sha256:<hex>`.
    pub program: String,
    /// The `repo=` name of the repository searched.
    pub repo: String,
    pub commit: String,
    /// The tool's completeness line (Tuft's `# coverage=… program=…`).
    pub trailer: String,
    /// Rows the view holds, and rows shown on this page.
    pub rows: usize,
    pub shown: usize,
    /// One citation token per cited span, in row order.
    pub cites: Vec<String>,
}

/// The run's ID when the KB's current-run pointer names a run that exists
/// and is not closed; `None` otherwise (and for a KB outside Git).
pub fn current(ws: &Workspace) -> Option<String> {
    let id = runs::current(&ws.repo.as_ref()?.git_dir.join("writfold"))?;
    let dir = ws.root.join("runs").join(&id);
    if !dir.join("manifest.json").is_file() {
        return None;
    }
    let state = |file: &str| {
        let text = std::fs::read_to_string(dir.join(file)).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        value.get("state")?.as_str().map(str::to_owned)
    };
    let state = state(runs::SIDECAR).or_else(|| state("manifest.json"));
    (state.as_deref() != Some("closed")).then_some(id)
}

/// Citation tokens for spans of files at `commit` of the repository in
/// `dir`, and the `repo=` name they carry.
pub struct Cited {
    pub repo: String,
    /// Whether `stale` finds the repository again from `repo=` (and the
    /// URL); when not, the tokens check as unverifiable until it is mapped.
    pub resolvable: bool,
    pub tokens: Vec<String>,
}

/// Cite `wanted` at `commit` of the repository in `dir`, as `cite make`
/// would cite those lines at that commit. `ws` supplies the repository map
/// that names the repository.
pub fn cite_spans(
    ws: Option<&Workspace>,
    dir: &Path,
    commit: &str,
    wanted: &[Span],
    now: i64,
) -> Result<Cited> {
    let dir = std::fs::canonicalize(dir)?;
    let git_dir = repos::git_dir(&dir)
        .ok_or_else(|| KbError::Refused(format!("{} is not a repository", dir.display())))?;
    let tool = GitTool::locate()?;
    let git = tool.admit()?;
    let url = url(&git, &git_dir, &dir)?;
    let (repo, resolvable) = repos::name_for(ws, &dir, &url);
    let seen = seen(now);
    let mut out = Vec::with_capacity(wanted.len());
    for spanned in spans(&git, &git_dir, commit, wanted)? {
        out.push(code_token(&Fields {
            repo: &repo,
            rev: commit,
            spanned: &spanned,
            sym: None,
            seen: &seen,
            ttl: None,
            url: &url,
            dirty: None,
        })?);
    }
    Ok(Cited {
        repo,
        resolvable,
        tokens: out,
    })
}

/// The receipt's bytes.
fn receipt(r: &Retrieval, run: &str, stamp: &str) -> String {
    let short = |text: &str| text.get(..12).unwrap_or(text).to_owned();
    let program = r.program.strip_prefix("sha256:").unwrap_or(&r.program);
    let mut out = format!(
        "---\nkind: retrieval\ntool: {}\nrun: {run}\nprogram: {}\nrepo: {}\ncommit: {}\nrows: {}\nshown: {}\ncites: {}\nrecorded: {stamp}\n---\n\n# Retrieval {} at {}@{}\n\ntrailer: `{}`\n",
        r.tool,
        r.program,
        r.repo,
        r.commit,
        r.rows,
        r.shown,
        r.cites.len(),
        short(program),
        r.repo,
        short(&r.commit),
        r.trailer
    );
    if !r.cites.is_empty() {
        out.push_str("\n## Citations\n\n");
        for token in &r.cites {
            out.push_str(&format!("- {token}\n"));
        }
    }
    out
}

/// Plan one receipt in the named or current run.
pub fn plan(session: &Session<'_>, r: &Retrieval) -> Result<Plan> {
    for (flag, text) in [
        ("tool", &r.tool),
        ("program", &r.program),
        ("repo", &r.repo),
        ("commit", &r.commit),
        ("trailer", &r.trailer),
    ] {
        if one_line(flag, text)? != *text || text.contains('`') {
            return Err(KbError::Usage(format!("{flag} must be one plain line")));
        }
    }
    if r.cites
        .iter()
        .any(|t| tokens(t).len() != 1 || t.contains('\n'))
    {
        return Err(KbError::Usage(
            "each cite must be one citation token".into(),
        ));
    }
    let rec = super::run::target(session, r.run.as_deref())?;
    if runs::state(session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    let stamp = time::stamp(session.now);
    let compact: String = stamp.chars().filter(|c| *c != '-' && *c != ':').collect();
    let mut salt = Sha256::new();
    salt.update(format!("{stamp}\0{}\0{}", std::process::id(), r.program).as_bytes());
    salt.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            .to_le_bytes(),
    );
    let name = format!("{compact}-{}", &crate::index::hex(&salt.finalize())[..6]);
    let path = format!("{}/{DIR}/{name}.md", runs::dir(&rec));
    Ok(Plan {
        ops: vec![Op::Create {
            path: path.clone(),
            bytes: receipt(r, &rec.id, &stamp).into_bytes(),
        }],
        show: None,
        notes: vec![format!("recorded {path}")],
        current: None,
        verb: "retrieval",
        args: vec![rec.id, name],
    })
}

/// The run's retrieval receipts, newest first: the newest few as rows (the
/// tool, program, commit and counts; the trailer and the first citations
/// as continuation lines), and how many there are.
pub fn rows(session: &Session<'_>, rec: &Rec) -> Result<(Vec<Row>, usize)> {
    let prefix = format!("{}/{DIR}/", runs::dir(rec));
    let width = i64::try_from(prefix.len()).unwrap_or(i64::MAX);
    let mut statement = session.conn.prepare_cached(
        "SELECT path FROM files WHERE substr(path, 1, ?1) = ?2 AND path LIKE '%.md' \
         ORDER BY path DESC",
    )?;
    let found = statement
        .query_map((width, &prefix), |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let total = found.len();
    let out = found.iter().take(SHOWN).map(|p| row(session, p)).collect();
    Ok((out, total))
}

fn row(session: &Session<'_>, path: &str) -> Row {
    let text = session.text(path).unwrap_or_default();
    let fields: Vec<(String, String)> = parse_frontmatter(&text)
        .map(|block| {
            block
                .entries()
                .map(|(k, v)| (k.to_owned(), v.trim().to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let get = |key: &str| {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map_or("?", |(_, v)| v.as_str())
    };
    let short = |text: &str| text.get(..12).unwrap_or(text).to_owned();
    let program = get("program");
    let mut row = Row {
        kind: "retrieval",
        id: crate::layout::stem(path).to_owned(),
        text: format!(
            "{} program={} {}@{} rows={} shown={} cites={}",
            get("tool"),
            short(program.strip_prefix("sha256:").unwrap_or(program)),
            get("repo"),
            short(get("commit")),
            get("rows"),
            get("shown"),
            get("cites"),
        ),
        path: Some(path.to_owned()),
        line: Some(1),
        ..Row::default()
    };
    let at = |line: usize| Some(u32::try_from(line + 1).unwrap_or(u32::MAX));
    if let Some((n, line)) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.starts_with("trailer: "))
    {
        let trailer = line["trailer: ".len()..].trim_matches('`').to_owned();
        row.more.push(more("trailer", trailer, path, at(n)));
    }
    let cites = tokens(&text);
    for cite in cites.iter().take(CITES_SHOWN) {
        let lines = match (cite.first, cite.last) {
            (Some(a), Some(b)) => format!(":{a}-{b}"),
            _ => String::new(),
        };
        let cited = cite.path.as_deref().unwrap_or_default();
        let text = format!("{cited}{lines}");
        row.more.push(more("cite", text, path, Some(cite.line)));
    }
    if cites.len() > CITES_SHOWN {
        let rest = format!("{} more citations", cites.len() - CITES_SHOWN);
        row.more.push(more("cite", rest, path, None));
    }
    row
}

fn more(label: &'static str, text: String, path: &str, line: Option<u32>) -> More {
    More {
        label,
        text,
        path: Some(path.to_owned()),
        line,
        flags: Vec::new(),
    }
}
