//! Edit receipts: one Markdown file per applied batch,
//! `runs/<id>/edits/<UTC compact time>-<6 hex>.md`, written create-new
//! through the write protocol. Front matter names the tool and its pinned
//! kernel, the run, the worktree, its repository, the run's base and the
//! worktree's HEAD, and the counts. The body lists each file with its
//! before and after blob IDs and Splicery's splice sites, then each recipe
//! verbatim with its SHA-256, then the preview diff. The prototype never
//! reads this directory; Writfold indexes the file as a document.
use super::plan::{FileEdit, Recipe};
use crate::emit::{More, Row};
use crate::error::Result;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use writfold_sources::frontmatter::parse_frontmatter;
use writfold_sources::splicery::identity;

/// The directory of a run that holds its edit receipts.
pub const DIR: &str = "edits";
/// Receipts `resume` shows, newest first.
const SHOWN: usize = 3;
/// Files shown under each receipt in `resume`.
const FILES_SHOWN: usize = 5;

/// What a receipt states besides the files and recipes.
pub struct Facts<'a> {
    pub run: &'a str,
    pub repo: &'a str,
    pub worktree: &'a str,
    pub base: Option<&'a str>,
    pub head: Option<&'a str>,
    pub recorded: &'a str,
}

/// A fence longer than any run of backticks in `text`.
fn fence(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    "`".repeat((longest + 1).max(4))
}

fn block(out: &mut String, info: &str, text: &str) {
    let fence = fence(text);
    out.push_str(&format!("{fence}{info}\n{text}"));
    if !text.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("{fence}\n"));
}

/// The line, in the receipt, of each file's entry (one-based), and the
/// receipt's bytes.
pub fn bytes(facts: &Facts<'_>, recipes: &[Recipe], files: &[FileEdit]) -> (Vec<u32>, String) {
    let short = |t: &str| t.get(..12).unwrap_or(t).to_owned();
    let id = identity();
    let splices: usize = files.iter().map(|f| f.sites.len()).sum();
    let mut out = format!(
        "---\nkind: edit\ntool: splicery\nprofile: {}\nkernel: {}\nrun: {}\nrepo: {}\nworktree: {}\nbase: {}\nhead: {}\nrecipes: {}\nfiles: {}\nsplices: {splices}\nrecorded: {}\n---\n\n# Edit at {}@{}\n\nApplied in the run's worktree by `writfold kb edit apply`. The result is an ordinary uncommitted Git change there; gates judge it afterwards.\n\n## Files\n\n",
        id.profile(),
        id.commit(),
        facts.run,
        facts.repo,
        facts.worktree,
        facts.base.unwrap_or_default(),
        facts.head.unwrap_or_default(),
        recipes.len(),
        files.len(),
        facts.recorded,
        facts.repo,
        short(facts.head.unwrap_or("unborn")),
    );
    let mut lines = Vec::new();
    for f in files {
        lines.push(u32::try_from(out.lines().count() + 1).unwrap_or(u32::MAX));
        let recipe = recipes.get(f.recipe).map_or("?", |r| r.name.as_str());
        out.push_str(&format!(
            "- `{}` before={} after={} splices={} removed={} inserted={} recipe={recipe}\n",
            f.path,
            f.before_blob,
            f.after_blob,
            f.sites.len(),
            f.removed(),
            f.inserted()
        ));
        for s in &f.sites {
            out.push_str(&format!("  - {}\n", site(s)));
        }
    }
    for r in recipes {
        out.push_str(&format!(
            "\n## Recipe {}\n\nsha256: {}\n\n",
            r.name, r.sha256
        ));
        block(&mut out, "splicery", &String::from_utf8_lossy(&r.bytes));
    }
    out.push_str("\n## Preview\n\n");
    let diff: String = files.iter().map(|f| f.diff.as_str()).collect();
    block(&mut out, "diff", &diff);
    (lines, out)
}

/// One splice site as the receipt and the answer state it.
pub fn site(s: &super::plan::Site) -> String {
    format!(
        "line {} bytes [{},{}) -> {} bytes, recipe [{},{})",
        s.line, s.lo, s.hi, s.inserted, s.origin.0, s.origin.1
    )
}

/// The run's edit receipts, newest first: the newest few as rows, and how
/// many there are.
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
        .map(|b| {
            b.entries()
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
    let head = get("head");
    let mut row = Row {
        kind: "edit",
        id: crate::layout::stem(path).to_owned(),
        text: format!(
            "splicery files={} splices={} at {}@{}",
            get("files"),
            get("splices"),
            get("repo"),
            head.get(..12).unwrap_or(head),
        ),
        path: Some(path.to_owned()),
        line: Some(1),
        ..Row::default()
    };
    let entries: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with("- `"))
        .collect();
    for (n, line) in entries.iter().take(FILES_SHOWN) {
        row.more.push(More {
            label: "file",
            text: summary(line),
            path: Some(path.to_owned()),
            line: u32::try_from(n + 1).ok(),
            flags: Vec::new(),
        });
    }
    if entries.len() > FILES_SHOWN {
        row.more.push(More {
            label: "file",
            text: format!("{} more files", entries.len() - FILES_SHOWN),
            path: Some(path.to_owned()),
            ..More::default()
        });
    }
    row
}

/// `- `p` before=B after=A splices=…` as `p B12→A12 splices=…`.
fn summary(line: &str) -> String {
    let body = line.trim_start_matches("- `");
    let (path, rest) = body.split_once('`').unwrap_or((body, ""));
    let mut parts = Vec::new();
    let (mut before, mut after) = ("?", "?");
    for word in rest.split_whitespace() {
        match word.split_once('=') {
            Some(("before", v)) => before = v,
            Some(("after", v)) => after = v,
            Some(("splices", _)) | Some(("recipe", _)) => parts.push(word),
            _ => {}
        }
    }
    let short = |t: &str| t.get(..12).unwrap_or(t).to_owned();
    format!(
        "{path} {}->{} {}",
        short(before),
        short(after),
        parts.join(" ")
    )
}
