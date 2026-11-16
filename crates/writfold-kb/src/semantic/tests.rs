use super::{chunks, scan};
use crate::error::Result;
use crate::verbs::find::Filter;
use rusqlite::Connection;

#[test]
fn section_identity_tracks_content_not_position() {
    let body = "---\nid: test\n---\n# Title\n## Decision\nStore plain text.\n## Reason\nIt survives a move.\n";
    let sections = crate::parse::sections(body);
    let first = chunks::split("Title", body, &sections, "model-a");
    assert_eq!((first[1].first, first[1].last), (5, 6));
    assert_eq!(first[1].text, "Title\n## Decision\nStore plain text.\n");
    let moved = body.replace("# Title\n", "# Title\n\n");
    let after = chunks::split("Title", &moved, &crate::parse::sections(&moved), "model-a");
    assert_eq!(first[1].hash, after[1].hash);
    assert_eq!(after[1].first, 6);
    let other = chunks::split("Title", body, &sections, "model-b");
    assert_ne!(first[1].hash, other[1].hash);
    let titled = chunks::split("Changed", body, &sections, "model-a");
    assert_ne!(first[1].hash, titled[1].hash);
    assert!(chunks::split("Empty", "", &[], "model-a").is_empty());
    let headingless = chunks::split("Title", "body\n", &[], "model-a");
    assert_eq!((headingless[0].first, headingless[0].last), (1, 1));
}

fn blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}

#[test]
fn cosine_rejects_corruption_and_excludes_zero_vectors() -> Result<()> {
    assert_eq!(scan::cosine(&[1.0, 0.0], &blob(&[2.0, 0.0]))?, Some(1.0));
    assert_eq!(scan::cosine(&[1.0, 0.0], &blob(&[0.0, 0.0]))?, None);
    assert!(scan::cosine(&[1.0, 0.0], &[0]).is_err());
    assert!(scan::cosine(&[1.0, 0.0], &blob(&[f32::NAN, 0.0])).is_err());
    assert!(scan::cosine(&[1.0, 0.0], &blob(&[f32::MAX, f32::MAX])).is_err());
    Ok(())
}

#[test]
fn filtered_scan_keeps_best_section_and_deterministic_ties() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE TABLE semantic_chunks(id TEXT, path TEXT, first INTEGER, last INTEGER, hash BLOB);
        CREATE TABLE semantic_vectors(hash BLOB, vector BLOB);
        CREATE TABLE records(id TEXT, kind TEXT, project TEXT);",
    )?;
    for (id, project, score) in [
        ("a", "other", 1.0),
        ("b", "wanted", 0.6),
        ("c", "wanted", 0.6),
    ] {
        conn.execute(
            "INSERT INTO records VALUES (?1, 'decision', ?2)",
            (id, project),
        )?;
        conn.execute(
            "INSERT INTO semantic_chunks VALUES (?1, ?2, 4, 8, ?3)",
            (id, format!("{project}/{id}.md"), id.as_bytes()),
        )?;
        conn.execute(
            "INSERT INTO semantic_vectors VALUES (?1, ?2)",
            (id.as_bytes(), blob(&[score, 0.8])),
        )?;
    }
    conn.execute(
        "INSERT INTO semantic_chunks VALUES ('b', 'wanted/b.md', 10, 15, x'64')",
        [],
    )?;
    conn.execute(
        "INSERT INTO semantic_vectors VALUES (x'64', ?1)",
        [blob(&[0.3, 0.9])],
    )?;
    let filter = Filter {
        kinds: vec!["decision".into()],
        project: Some("wanted".into()),
    };
    let hits = scan::scan(&conn, &[1.0, 0.0], &filter, 2)?;
    assert_eq!(
        hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
        ["b", "c"]
    );
    assert_eq!((hits[0].first, hits[0].last), (4, 8));
    assert_eq!(scan::scan(&conn, &[1.0, 0.0], &filter, 1)?[0].id, "b");
    Ok(())
}

#[cfg(feature = "semantic-embedded")]
#[test]
fn cache_reuses_moved_chunks_and_drops_deleted_sources() -> Result<()> {
    use super::{cache, model};
    use crate::index::{Pin, build};
    let path = "notes/a.md";
    let text = "# Storage\n## Choice\nKeep plain text.\n## Reason\nIt survives moves.\n";
    let input = |text: &str| -> Result<build::Input> {
        Ok(build::Input {
            path: path.into(),
            kind: crate::layout::classify(path)
                .ok_or_else(|| crate::error::KbError::Usage("fixture path".into()))?,
            bytes: Ok(text.as_bytes().to_vec()),
        })
    };
    let mut conn = Connection::open_in_memory()?;
    build::build(&mut conn, vec![input(text)?], &[])?;
    let mut pin = Pin {
        head: None,
        dirty: 0,
        digest: "before".into(),
    };
    let model = model::get()?;
    assert_eq!(cache::sync(&conn, &pin, model)?, 3);
    assert_eq!(cache::sync(&conn, &pin, model)?, 0);
    let tx = conn.transaction()?;
    build::delete_file(&tx, path)?;
    build::insert_file(&tx, input(&text.replace("# Storage\n", "# Storage\n\n"))?)?;
    crate::index::derive::phase_b(&tx)?;
    tx.commit()?;
    pin.digest = "moved".into();
    // Only the changed preamble is embedded; both sections reuse their bytes.
    assert_eq!(cache::sync(&conn, &pin, model)?, 1);
    let span: u32 = conn.query_row(
        "SELECT first FROM semantic_chunks WHERE first > 2 ORDER BY first LIMIT 1",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(span, 3);
    let tx = conn.transaction()?;
    build::delete_file(&tx, path)?;
    crate::index::derive::phase_b(&tx)?;
    tx.commit()?;
    pin.digest = "deleted".into();
    cache::sync(&conn, &pin, model)?;
    let count: u32 = conn.query_row("SELECT count(*) FROM semantic_chunks", [], |r| r.get(0))?;
    assert_eq!(count, 0);
    conn.execute("INSERT INTO meta VALUES ('pin', 'different')", [])?;
    assert!(cache::sync(&conn, &pin, model).is_err());
    Ok(())
}
