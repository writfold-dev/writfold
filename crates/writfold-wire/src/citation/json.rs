use super::{
    CitationDescriptor, CitationWireError, GIT_BLOB_PROFILE, NAMESPACE, Result, SCHEMA, algorithm,
};
use serde::Serialize;
use std::io::{self, Write};

// Fields are in canonical Unicode-key order. Values borrow checked immutable
// metadata: no cloned JSON tree or copied source preimage is created to encode.
#[derive(Serialize)]
struct NativeRevision<'a> {
    algorithm: &'static str,
    value: &'a str,
}

#[derive(Serialize)]
struct Source<'a> {
    blob: NativeRevision<'a>,
    capture_digest: &'a str,
    path: &'a str,
    profile: &'static str,
    realm: &'a str,
    repository: &'a str,
    revision: NativeRevision<'a>,
}

#[derive(Serialize)]
struct Selection {
    first: u64,
    last: u64,
    unit: &'static str,
}

#[derive(Serialize)]
struct Envelope<'a> {
    context: &'a str,
    id: &'a str,
    kind: &'static str,
    namespace: &'static str,
    release: &'a str,
    representation: Source<'a>,
    role: &'a str,
    schema: u64,
    selection_digest: &'a str,
    selector: Selection,
}

#[derive(Clone, Copy)]
enum Failure {
    Limit,
    Allocation,
}

struct Bounded {
    bytes: Vec<u8>,
    maximum: usize,
    failure: Option<Failure>,
}

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            self.failure = Some(Failure::Limit);
            return Err(io::Error::other("citation output budget"));
        }
        let required = self.bytes.len() + bytes.len();
        if required > self.bytes.capacity() {
            let capacity = required
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.maximum);
            if self
                .bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .is_err()
            {
                self.failure = Some(Failure::Allocation);
                return Err(io::Error::other("citation output allocation"));
            }
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode(descriptor: &CitationDescriptor, maximum: usize) -> Result<Vec<u8>> {
    let reference = descriptor.source.reference();
    let format = algorithm(descriptor.source.format());
    let envelope = Envelope {
        context: descriptor.context.as_str(),
        id: descriptor.id.as_str(),
        kind: "binding",
        namespace: NAMESPACE,
        release: descriptor.release.as_str(),
        representation: Source {
            blob: NativeRevision {
                algorithm: format,
                value: descriptor.source.blob().as_str(),
            },
            capture_digest: reference.content().as_str(),
            path: reference.path(),
            profile: GIT_BLOB_PROFILE,
            realm: reference.realm().as_str(),
            repository: reference.repository().as_str(),
            revision: NativeRevision {
                algorithm: format,
                value: reference.revision().as_str(),
            },
        },
        role: descriptor.role.as_str(),
        schema: SCHEMA,
        selection_digest: descriptor.selection_digest.as_str(),
        selector: Selection {
            first: descriptor.selector.first().get(),
            last: descriptor.selector.last().get(),
            unit: "byte",
        },
    };
    let mut output = Bounded {
        bytes: Vec::new(),
        maximum,
        failure: None,
    };
    if serde_json::to_writer(&mut output, &envelope).is_err() {
        return Err(match output.failure {
            Some(Failure::Limit) => CitationWireError::Wire(crate::Error::Limit {
                budget: crate::Budget::Bytes,
                offset: output.bytes.len(),
            }),
            Some(Failure::Allocation) => CitationWireError::Allocation,
            None => CitationWireError::Encoding,
        });
    }
    Ok(output.bytes)
}
