use super::{
    CitationDescriptor, CitationWireError, GIT_BLOB_PROFILE, NAMESPACE, Result, SCHEMA, algorithm,
};
use crate::rpc::response::bounded::{Bounded, Failure};
use serde::Serialize;

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
    let mut output = Bounded::new(maximum);
    if let Err(failure) = output.json(&envelope) {
        return Err(match failure {
            Failure::Limit => CitationWireError::Wire(crate::Error::Limit {
                budget: crate::Budget::Bytes,
                offset: output.len(),
            }),
            Failure::Allocation => CitationWireError::Allocation,
            Failure::Encoding => CitationWireError::Encoding,
        });
    }
    Ok(output.into_bytes())
}
