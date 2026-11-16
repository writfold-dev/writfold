//! Section spans refer to the exact source text retained by FTS, not disk.
use crate::error::Result;
use crate::model::Section;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

pub(super) struct Chunk {
    pub first: u32,
    pub last: u32,
    pub text: String,
    pub hash: Vec<u8>,
}

pub(super) fn sections(conn: &Connection, path: &str) -> Result<Vec<Section>> {
    let mut statement = conn.prepare_cached(
        "SELECT name, line_first, line_last FROM sections WHERE path = ?1 ORDER BY ord",
    )?;
    Ok(statement
        .query_map([path], |row| {
            Ok(Section {
                name: row.get(0)?,
                first: row.get(1)?,
                last: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

/// Front matter is metadata, not a section. Headingless files have one
/// body chunk; prose before the first section gets its own chunk.
pub(super) fn split(title: &str, body: &str, sections: &[Section], model: &str) -> Vec<Chunk> {
    let lines: Vec<&str> = body.lines().collect();
    let start = crate::parse::mention::body_start(body);
    let end = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    let preamble_end = sections.first().map_or(end, |s| s.first.saturating_sub(1));
    let spans =
        std::iter::once((start, preamble_end)).chain(sections.iter().map(|s| (s.first, s.last)));
    let mut chunks = Vec::new();
    for (first, last) in spans {
        if first == 0 || first > last {
            continue;
        }
        let Some(selected) = lines.get(first as usize - 1..last as usize) else {
            continue;
        };
        if selected.iter().all(|line| line.trim().is_empty()) {
            continue;
        }
        let mut text = String::with_capacity(
            title.len() + selected.iter().map(|l| l.len() + 1).sum::<usize>() + 1,
        );
        text.push_str(title);
        text.push('\n');
        for line in selected {
            text.push_str(line);
            text.push('\n');
        }
        let mut digest = Sha256::new();
        digest.update(b"writfold-semantic-chunk/1\0");
        digest.update(model.as_bytes());
        digest.update([0]);
        digest.update(text.as_bytes());
        chunks.push(Chunk {
            first,
            last,
            text,
            hash: digest.finalize().to_vec(),
        });
    }
    chunks
}
