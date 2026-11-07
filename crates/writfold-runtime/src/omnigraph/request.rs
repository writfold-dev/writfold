use super::{NativeError, http};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::{GraphCommitId, GraphId};

/// Closed scalar parameter profile; nested JSON and non-finite floats are refused.
#[derive(Clone, Copy, Debug)]
pub enum Scalar<'a> {
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub value: Scalar<'a>,
}

/// Inputs to one named stored query; the method separately fixes read or
/// conditional-write semantics. These inputs are not an approval capability.
#[derive(Clone, Copy, Debug)]
pub struct StoredQuery<'a> {
    pub actor: &'a str,
    pub graph: &'a GraphId,
    pub name: &'a str,
    pub params: &'a [Parameter<'a>],
}
pub(super) enum Target<'a> {
    Snapshot(&'a GraphCommitId),
    Branch(&'a str),
}

pub(super) fn query_body(
    target: Target<'_>,
    params: &[Parameter<'_>],
    mutation: bool,
    limit: u64,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, NativeError> {
    http::stop(deadline, cancelled)?;
    let mut body = BoundedBody::new(limit, deadline, cancelled);
    body.write_all(b"{\"params\":{")?;
    // This profile is intentionally scalar and small; bound the key count before
    // duplicate checking, independent of the serialized-byte budget.
    if params.len() > 256 {
        return Err(NativeError::Input("at most 256 scalar parameters"));
    }
    for (index, parameter) in params.iter().enumerate() {
        if parameter.name.is_empty()
            || parameter.name.len() > 128
            || params[..index]
                .iter()
                .any(|previous| previous.name == parameter.name)
        {
            return Err(NativeError::Input("invalid or duplicate parameter name"));
        }
        if index != 0 {
            body.write_all(b",")?;
        }
        serde_json::to_writer(&mut body, parameter.name)?;
        body.write_all(b":")?;
        match parameter.value {
            Scalar::String(value) => {
                // Refuse an impossible lower bound before JSON scans the string
                // for escaping; an oversized caller input cannot bypass time
                // bounds through a single serializer scan.
                if value.len() as u64 > limit {
                    return Err(NativeError::Budget("parameter bytes"));
                }
                serde_json::to_writer(&mut body, value)?;
            }
            Scalar::Integer(value) => serde_json::to_writer(&mut body, &value)?,
            Scalar::Float(value) => {
                if !value.is_finite() {
                    return Err(NativeError::Input("non-finite parameter"));
                }
                serde_json::to_writer(&mut body, &value)?;
            }
            Scalar::Boolean(value) => serde_json::to_writer(&mut body, &value)?,
            Scalar::Null => body.write_all(b"null")?,
        }
    }
    body.write_all(b"},")?;
    match target {
        Target::Snapshot(pin) => {
            if mutation {
                return Err(NativeError::Input("mutation cannot target snapshot"));
            }
            body.write_all(b"\"snapshot\":")?;
            serde_json::to_writer(&mut body, pin.as_str())?;
        }
        Target::Branch(branch) => {
            http::segment(branch)?;
            body.write_all(b"\"branch\":")?;
            serde_json::to_writer(&mut body, branch)?;
        }
    }
    body.write_all(if mutation {
        b",\"expect_mutation\":true}"
    } else {
        b",\"expect_mutation\":false}"
    })?;
    Ok(body.bytes)
}

pub(super) struct BoundedBody<'a> {
    pub(super) bytes: Vec<u8>,
    limit: u64,
    deadline: Instant,
    cancelled: &'a AtomicBool,
}
impl<'a> BoundedBody<'a> {
    pub(super) fn new(limit: u64, deadline: Instant, cancelled: &'a AtomicBool) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            deadline,
            cancelled,
        }
    }
}
impl Write for BoundedBody<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        http::stop(self.deadline, self.cancelled).map_err(std::io::Error::other)?;
        if bytes.len() as u64 > self.limit.saturating_sub(self.bytes.len() as u64) {
            return Err(std::io::Error::other(
                "native request encoding exceeds byte budget",
            ));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(std::io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
