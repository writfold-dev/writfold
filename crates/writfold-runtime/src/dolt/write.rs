//! Closed native vocabulary for the realm's single Dolt writer.
//!
//! Callers never supply SQL. Commit identities are typed base32 tokens and every
//! row value or commit message is a hex literal, so no input reaches the SQL
//! grammar. A write is one `dolt sql` process running one explicit transaction:
//! a guard checks the expected-old head and the realm identity row, the row
//! changes apply, a closure guard refuses dangling edges, and `DOLT_COMMIT`
//! publishes. A failed guard raises a
//! native error, which aborts the process and rolls the transaction back.
use super::DoltError;
use std::fmt::Write as _;
use writfold_domain::DoltIntegrationCommitId;

/// The one conditional head the writer advances: the checked-out branch.
pub const WRITE_HEAD: &str = "main";
/// The whole script is one argv string, which Linux bounds at 128 KiB.
const SCRIPT_BYTES: usize = 120 * 1024;
const AUTHOR: &str = "Writfold Writer <writer@writfold.invalid>";

/// A rendered statement group from the closed writer vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltSql(String);

/// One row change. `id`, `kind` and `label` are arbitrary UTF-8 and are always
/// rendered as hex literals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoltNodeChange<'a> {
    Put {
        id: &'a str,
        kind: &'a str,
        label: &'a str,
    },
    Delete {
        id: &'a str,
    },
}

/// The single `wf_reader_meta` row the database must carry, compared byte for
/// byte (as hex) so no collation can equate different identities.
#[derive(Clone, Copy, Debug)]
pub struct DoltRealmRow<'a> {
    pub realm: &'a str,
    pub kb: &'a str,
    pub database: &'a str,
    pub epoch: &'a str,
    pub schema_digest: &'a str,
    pub query_digest: &'a str,
    pub storage_profile: &'a str,
    pub producer: &'a str,
}

/// One conditional commit on `WRITE_HEAD`.
#[derive(Clone, Copy, Debug)]
pub struct DoltWrite<'a> {
    pub expected: &'a DoltIntegrationCommitId,
    pub realm: DoltRealmRow<'a>,
    /// Also create `wf-retained/<expected>` (completing an interrupted write).
    pub retain_expected: bool,
    pub changes: &'a [DoltNodeChange<'a>],
    pub message: &'a str,
}

impl DoltSql {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    /// Head commit, active branch, its retained root and pending changes.
    pub fn head_probe() -> Self {
        Self(format!(
            "SELECT HASHOF('{WRITE_HEAD}') AS head, ACTIVE_BRANCH() AS branch, \
             (SELECT COUNT(*) FROM dolt_branches WHERE name = CONCAT('wf-retained/', \
             HASHOF('{WRITE_HEAD}'))) AS retained, (SELECT COUNT(*) FROM dolt_status) AS pending"
        ))
    }

    /// Commits on the head's history whose parent is `expected` and whose
    /// message starts with `marker` (an ASCII write identity).
    pub fn applied_probe(
        expected: &DoltIntegrationCommitId,
        marker: &str,
    ) -> Result<Self, DoltError> {
        if !marker.is_ascii() || marker.is_empty() {
            return Err(DoltError::QueryContract);
        }
        Ok(Self(format!(
            "SELECT l.commit_hash AS native_commit FROM dolt_log AS l JOIN \
             dolt_commit_ancestors AS a ON a.commit_hash = l.commit_hash WHERE \
             LEFT(l.message, {}) = {} AND a.parent_hash = '{}' ORDER BY l.commit_hash LIMIT 2",
            marker.len(),
            literal(marker),
            expected.as_str()
        )))
    }

    /// The parent of the earliest commit on the head's history whose message
    /// starts with `prefix` (the writer's marker family): the realm's origin
    /// before its first write. No row means no such commit exists.
    pub fn origin_probe(prefix: &str) -> Result<Self, DoltError> {
        if !prefix.is_ascii() || prefix.is_empty() {
            return Err(DoltError::QueryContract);
        }
        let (length, prefix) = (prefix.len(), literal(prefix));
        Ok(Self(format!(
            "SELECT a.parent_hash AS native_commit FROM dolt_log AS l JOIN \
             dolt_commit_ancestors AS a ON a.commit_hash = l.commit_hash WHERE \
             LEFT(l.message, {length}) = {prefix} AND a.parent_hash NOT IN (SELECT \
             p.commit_hash FROM dolt_log AS p WHERE LEFT(p.message, {length}) = {prefix}) \
             ORDER BY a.parent_hash LIMIT 2"
        )))
    }

    /// Create the retention root `wf-retained/<head>` for the current write
    /// head, guarded in the same process: the head must still be `head` and
    /// the working set clean. Changes no row and does not move the head. The
    /// caller first observes that the root is absent, under the same lease.
    pub fn retain_head(head: &DoltIntegrationCommitId) -> Self {
        let head = head.as_str();
        Self(format!(
            "SELECT IF(ACTIVE_BRANCH() = '{WRITE_HEAD}' AND HASHOF('{WRITE_HEAD}') = '{head}' \
             AND (SELECT COUNT(*) FROM dolt_status) = 0, 1, \
             JSON_EXTRACT('writfold-head-guard', '$')) AS guard; \
             CALL DOLT_BRANCH('wf-retained/{head}', '{head}');"
        ))
    }

    /// The single-transaction conditional commit. Oversized scripts refuse.
    pub fn write(write: &DoltWrite<'_>) -> Result<Self, DoltError> {
        let expected = write.expected.as_str();
        let mut sql = guard(write)?;
        if write.retain_expected {
            push(
                &mut sql,
                format_args!(" CALL DOLT_BRANCH('wf-retained/{expected}', '{expected}');"),
            )?;
        }
        for change in write.changes {
            match change {
                DoltNodeChange::Put { id, kind, label } => push(
                    &mut sql,
                    format_args!(
                        " INSERT INTO wf_nodes (id,kind,label,home_path) VALUES ({},{},{},NULL) \
                         ON DUPLICATE KEY UPDATE kind = VALUES(kind), label = VALUES(label);",
                        literal(id),
                        literal(kind),
                        literal(label)
                    ),
                )?,
                DoltNodeChange::Delete { id } => push(
                    &mut sql,
                    format_args!(" DELETE FROM wf_nodes WHERE id = {};", literal(id)),
                )?,
            }
            if sql.len() > SCRIPT_BYTES {
                return Err(budget());
            }
        }
        push(
            &mut sql,
            format_args!(
                " SELECT IF((SELECT COUNT(*) FROM wf_edges AS e WHERE NOT EXISTS (SELECT 1 FROM \
                 wf_nodes AS n WHERE n.id = e.source) OR NOT EXISTS (SELECT 1 FROM wf_nodes AS n \
                 WHERE n.id = e.target)) = 0, 1, JSON_EXTRACT('writfold-closure-guard', '$')) \
                 AS closure; CALL DOLT_COMMIT('-A', '--allow-empty', '--author', '{AUTHOR}', \
                 '-m', {}); CALL DOLT_BRANCH(CONCAT('wf-retained/', HASHOF('HEAD')), \
                 HASHOF('HEAD'));",
                literal(write.message)
            ),
        )?;
        if sql.len() > SCRIPT_BYTES {
            return Err(budget());
        }
        Ok(Self(sql))
    }
}

/// `START TRANSACTION` and the guard: the head is the expected-old commit, the
/// working set is clean and the realm row is exactly `write.realm`.
fn guard(write: &DoltWrite<'_>) -> Result<String, DoltError> {
    let expected = write.expected.as_str();
    let mut sql = format!(
        "START TRANSACTION; SELECT IF(ACTIVE_BRANCH() = '{WRITE_HEAD}' AND \
         HASHOF('{WRITE_HEAD}') = '{expected}' AND HASHOF('HEAD') = '{expected}' AND \
         (SELECT COUNT(*) FROM dolt_status) = 0 AND \
         (SELECT COUNT(*) FROM wf_reader_meta) = 1 AND \
         (SELECT COUNT(*) FROM wf_reader_meta WHERE id = 1"
    );
    let row = write.realm;
    for (column, value) in [
        ("realm", row.realm),
        ("kb", row.kb),
        ("database_id", row.database),
        ("epoch", row.epoch),
        ("schema_digest", row.schema_digest),
        ("query_digest", row.query_digest),
        ("storage_profile", row.storage_profile),
        ("producer", row.producer),
    ] {
        push(
            &mut sql,
            format_args!(" AND HEX({column}) = '{}'", hex(value)),
        )?;
    }
    push(
        &mut sql,
        format_args!(") = 1, 1, JSON_EXTRACT('writfold-head-guard', '$')) AS guard;"),
    )?;
    Ok(sql)
}

fn push(sql: &mut String, part: std::fmt::Arguments<'_>) -> Result<(), DoltError> {
    sql.write_fmt(part).map_err(|_| DoltError::QueryContract)
}

fn budget() -> DoltError {
    DoltError::Budget {
        stream: "write script",
        limit: SCRIPT_BYTES as u64,
    }
}

/// `CONVERT(X'..' USING utf8mb4)`: arbitrary UTF-8 without SQL quoting.
fn literal(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }
    format!("CONVERT(X'{}' USING utf8mb4)", hex(value))
}

/// Uppercase hex, as SQL `HEX()` renders bytes.
fn hex(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut text = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_render_only_as_hex_literals() -> Result<(), Box<dyn std::error::Error>> {
        let expected: DoltIntegrationCommitId = "0123456789abcdefghijklmnopqrstuv".parse()?;
        let changes = [
            DoltNodeChange::Put {
                id: "a'); DROP TABLE wf_nodes; --",
                kind: "k",
                label: "",
            },
            DoltNodeChange::Delete { id: "b" },
        ];
        let realm = DoltRealmRow {
            realm: "r",
            kb: "k",
            database: "d",
            epoch: "e",
            schema_digest: "s",
            query_digest: "q",
            storage_profile: "p",
            producer: "x",
        };
        let write = DoltWrite {
            expected: &expected,
            realm,
            retain_expected: true,
            changes: &changes,
            message: "m'",
        };
        let sql = DoltSql::write(&write)?;
        assert!(!sql.as_str().contains("DROP"));
        assert!(sql.as_str().contains("CONVERT(X'6D27' USING utf8mb4)"));
        assert!(sql.as_str().contains("HEX(realm) = '72'"));
        assert!(
            sql.as_str()
                .contains("'wf-retained/0123456789abcdefghijklmnopqrstuv'")
        );
        let label = "x".repeat(SCRIPT_BYTES);
        let large = [DoltNodeChange::Put {
            id: "a",
            kind: "k",
            label: &label,
        }];
        let oversized = DoltWrite {
            changes: &large,
            ..write
        };
        assert!(matches!(
            DoltSql::write(&oversized),
            Err(DoltError::Budget { .. })
        ));
        assert!(DoltSql::applied_probe(&expected, "é").is_err());
        assert!(DoltSql::origin_probe("").is_err());
        let origin = DoltSql::origin_probe("w'")?;
        assert!(origin.as_str().contains("CONVERT(X'7727' USING utf8mb4)"));
        assert!(!origin.as_str().contains("w'"));
        Ok(())
    }
}
