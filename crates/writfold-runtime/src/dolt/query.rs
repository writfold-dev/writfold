use super::DoltError;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::sync::LazyLock;
use writfold_domain::{DoltIntegrationCommitId, IdentityError, QueryContractDigest, RowLimit};

/// Canonical UTF-8 template material, including each newline. Both rendering and
/// the query identity consume this value; no separately maintained SQL exists.
pub const QUERY_CONTRACT: &str = concat!(
    "SELECT id,HASHOF('{commit}') AS native_commit,realm,kb,database_id,epoch,schema_digest,query_digest,storage_profile,producer FROM wf_reader_meta AS OF '{commit}' ORDER BY id LIMIT 2\n",
    "SELECT HASHOF('wf-retained/{commit}') AS native_commit\n",
    "SELECT id,kind,label,COALESCE(home_path,'') AS home_path,home_path IS NULL AS home_is_null FROM wf_nodes AS OF '{commit}' ORDER BY BINARY id LIMIT {limit}\n",
    "SELECT id,source,target,kind FROM wf_edges AS OF '{commit}' ORDER BY BINARY id LIMIT {limit}\n",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoltQuery {
    Metadata,
    RetentionRoot,
    Nodes,
    Edges,
}

impl DoltQuery {
    pub(super) fn render(
        self,
        commit: &DoltIntegrationCommitId,
        rows: RowLimit,
    ) -> Result<String, DoltError> {
        let index = match self {
            Self::Metadata => 0,
            Self::RetentionRoot => 1,
            Self::Nodes => 2,
            Self::Edges => 3,
        };
        let template = QUERY_CONTRACT
            .lines()
            .nth(index)
            .ok_or(DoltError::QueryContract)?;
        let limit = rows.get().checked_add(1).ok_or(DoltError::RowLimit)?;
        let mut sql = String::with_capacity(template.len() + 64);
        let mut remainder = template;
        while let Some((prefix, token)) = remainder.split_once('{') {
            sql.push_str(prefix);
            if let Some(suffix) = token.strip_prefix("commit}") {
                sql.push_str(commit.as_str());
                remainder = suffix;
            } else if let Some(suffix) = token.strip_prefix("limit}") {
                write!(&mut sql, "{limit}").map_err(|_| DoltError::QueryContract)?;
                remainder = suffix;
            } else {
                return Err(DoltError::QueryContract);
            }
        }
        sql.push_str(remainder);
        Ok(sql)
    }
}

pub fn query_contract_digest() -> Result<&'static QueryContractDigest, IdentityError> {
    static DIGEST: LazyLock<Result<QueryContractDigest, IdentityError>> = LazyLock::new(|| {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(64);
        for byte in Sha256::digest(QUERY_CONTRACT.as_bytes()) {
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
        QueryContractDigest::try_from(encoded)
    });
    DIGEST.as_ref().map_err(Clone::clone)
}
