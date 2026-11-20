//! Backlinks, the answering half (stores-next spec S3): `context` and
//! `uses` end with a `material` section listing the store directories
//! whose `README.md` claims to serve the record. Whether a claim lists a
//! directory under a record is [`writfold_core::store::claim`] (proved):
//! the record's own ID, or an alias no other record has, read from a
//! declared, readable store. A row is the store's claim, never a KB fact:
//! it is flagged `store-claim`, and no cover row or assumption can cite it.
//! Reading the stores is [`crate::store::material`]; the lookups, row
//! order, text and JSON here are tested, not proved.
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::Rec;
use crate::session::Session;
use crate::store::material::{self, Entry, Read};
use rusqlite::Connection;
use serde_json::json;
use writfold_core::store::{Claim, claim};

/// The record a subject names, with the kernel's verdict.
fn judge(conn: &Connection, attached: bool, subject: &str) -> Result<(Claim, Option<String>)> {
    let exact: bool = conn
        .prepare_cached("SELECT EXISTS(SELECT 1 FROM records WHERE id = ?1)")?
        .query_row([subject], |r| r.get(0))?;
    let ids: Vec<String> = conn
        .prepare_cached(
            "SELECT DISTINCT record_id FROM aliases WHERE alias = ?1 ORDER BY record_id",
        )?
        .query_map([subject], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let verdict = claim(attached, exact, ids.len());
    let named = match (&verdict, exact) {
        (Claim::Listed, true) => Some(subject.to_owned()),
        (Claim::Listed, false) => ids.into_iter().next(),
        _ => None,
    };
    Ok((verdict, named))
}

/// The stores' reads, or none when no store is declared (no Git spawn).
/// A declaration that cannot be read is a note, never a failed verb.
pub fn reads(session: &Session<'_>, answer: &mut Answer) -> Result<Vec<Read>> {
    let stores = match crate::store::read(&session.ws.root) {
        Ok(stores) => stores,
        Err(e) => {
            answer
                .notes
                .push(format!("material: store declarations not read ({e})"));
            return Ok(Vec::new());
        }
    };
    if stores.is_empty() {
        return Ok(Vec::new());
    }
    material::read_all(session.git.as_ref(), session.ws, &session.conn, &stores)
}

fn row(read: &Read, entry: &Entry) -> Row {
    let commit = read.commit.clone().unwrap_or_default();
    let readme = match entry.dir.as_str() {
        "" => "README.md".to_owned(),
        dir => format!("{dir}/README.md"),
    };
    let mut flags = vec!["store-claim".to_owned()];
    if read.other_origin.is_some() {
        flags.push("origin-differs".into());
    }
    let more = |label, text: String| More {
        label,
        text,
        ..More::default()
    };
    Row {
        kind: "material",
        id: format!("{}:{}", read.name, entry.dir),
        text: entry.heading.clone().unwrap_or_else(|| entry.dir.clone()),
        flags,
        section: Some("material"),
        more: vec![
            more("at", format!("{}@{commit}:{readme}", read.name)),
            more("via", format!("subject: {}", entry.subject)),
        ],
        extra: vec![
            ("store", json!(read.name)),
            ("commit", json!(commit)),
            ("dir", json!(entry.dir)),
            ("subject", json!(entry.subject)),
            ("claim", json!("store")),
        ],
        ..Row::default()
    }
}

/// End `answer` with the `material` rows for `rec`, the notes for stores
/// that were not read, the `stores` trailer and `counts.material`. Nothing
/// at all when no store is declared.
pub fn append(session: &Session<'_>, rec: &Rec, answer: &mut Answer) -> Result<()> {
    let reads = reads(session, answer)?;
    if reads.is_empty() {
        return Ok(());
    }
    let mut rows = Vec::new();
    for read in &reads {
        let mut mine: Vec<&Entry> = Vec::new();
        for entry in &read.entries {
            let (verdict, named) = judge(&session.conn, read.attached(), &entry.subject)?;
            if matches!(verdict, Claim::Listed) && named.as_deref() == Some(rec.id.as_str()) {
                mine.push(entry);
            }
        }
        mine.sort_by(|a, b| b.dir.cmp(&a.dir));
        mine.dedup_by(|a, b| a.dir == b.dir);
        rows.extend(mine.into_iter().map(|e| row(read, e)));
        notes(read, answer);
    }
    answer.counts.push(("material", rows.len()));
    answer.rows.extend(rows);
    answer.trailer_json.push(("stores", trailer(&reads)));
    Ok(())
}

fn notes(read: &Read, answer: &mut Answer) {
    let name = &read.name;
    match read.state {
        "read" => {}
        "over-limit" => answer.notes.push(format!(
            "material: store {name} past its limits ({} README.md files, {} KiB each); its rows are incomplete",
            material::MAX_FILES,
            material::MAX_BYTES / 1024
        )),
        state => answer.notes.push(format!(
            "material: store {name} not read ({state}); writfold kb repos"
        )),
    }
    if let (Some(head), Some(commit)) = (&read.head, &read.commit) {
        answer.notes.push(format!(
            "material: store {name} read at {}; its HEAD {} differs, and material not pushed is not listed",
            short(commit),
            short(head)
        ));
    }
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn trailer(reads: &[Read]) -> serde_json::Value {
    let stores: Vec<_> = (reads.iter())
        .map(|r| json!({"name": r.name, "state": r.state, "commit": r.commit, "via": r.via}))
        .collect();
    json!(stores)
}

/// `kb repos` per store: how many directories are listed, ambiguous or
/// unknown, and up to five of the last two.
pub fn tally(conn: &Connection, read: &Read) -> Result<(usize, usize, usize, Vec<String>)> {
    let (mut listed, mut ambiguous, mut unknown, mut named) = (0, 0, 0, Vec::new());
    for entry in &read.entries {
        let (verdict, _) = judge(conn, read.attached(), &entry.subject)?;
        let word = match verdict {
            Claim::Listed => {
                listed += 1;
                continue;
            }
            Claim::Ambiguous => {
                ambiguous += 1;
                "ambiguous"
            }
            Claim::Unknown | Claim::NotAStore => {
                unknown += 1;
                "unknown"
            }
        };
        if named.len() < 5 {
            named.push(format!(
                "{word} {}:{} (subject: {})",
                read.name, entry.dir, entry.subject
            ));
        }
    }
    Ok((listed, ambiguous, unknown, named))
}
