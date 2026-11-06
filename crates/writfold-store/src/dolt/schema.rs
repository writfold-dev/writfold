use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use writfold_domain::{IdentityError, SchemaDigest};

/// Exact physical provisioning contract for `writfold-dolt-integration/1`.
/// The reader never executes this DDL or creates/migrates a database.
pub const SCHEMA_SQL: &str = "CREATE TABLE wf_reader_meta (\n\
    id TINYINT PRIMARY KEY,\n\
    realm TEXT NOT NULL,\n\
    kb TEXT NOT NULL,\n\
    database_id TEXT NOT NULL,\n\
    epoch TEXT NOT NULL,\n\
    schema_digest VARCHAR(64) NOT NULL,\n\
    query_digest VARCHAR(64) NOT NULL,\n\
    storage_profile TEXT NOT NULL,\n\
    producer VARCHAR(64) NOT NULL\n\
);\n\
CREATE TABLE wf_nodes (\n\
    id VARCHAR(600) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin PRIMARY KEY,\n\
    kind VARCHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,\n\
    label LONGTEXT CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,\n\
    home_path LONGTEXT CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL\n\
);\n\
CREATE TABLE wf_edges (\n\
    id VARCHAR(600) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin PRIMARY KEY,\n\
    source VARCHAR(600) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,\n\
    target VARCHAR(600) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,\n\
    kind VARCHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL\n\
);\n";

/// Digest of the exact UTF-8 provisioning contract, including its final newline.
pub fn schema_digest() -> Result<&'static SchemaDigest, IdentityError> {
    static DIGEST: LazyLock<Result<SchemaDigest, IdentityError>> = LazyLock::new(|| {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(64);
        for byte in Sha256::digest(SCHEMA_SQL.as_bytes()) {
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
        SchemaDigest::try_from(encoded)
    });
    DIGEST.as_ref().map_err(Clone::clone)
}
