//! Edit-flow ledger records for `writfold-trial-worktree/1`, and their pure
//! validator. One run is one stream: exactly `Prepared`, then `Approved`, then
//! `Applied`. Each record names its retained material by digest and length and
//! must carry that material in its closure. The validator checks order and
//! bindings only; signatures, grants and the file guard are checked by `apply`
//! against the retained bytes before `Applied` is appended.
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_kernel::ledger::{Record, Validator};
use writfold_wire::Limits;
use writfold_wire::engineering::cbor::{self, Decoder, Value as C};
use writfold_wire::engineering::ledger_record::{self as codec, Part};

/// A retained object: its digest and exact byte length.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Blob {
    pub digest: BlobDigest,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Prepared {
        path: String,
        base: String,
        source: Blob,
        recipe: Blob,
        candidate: Blob,
        report: Blob,
        prepared: [u8; 32],
    },
    Approved {
        grant: Blob,
        approval: Blob,
    },
    Applied {
        candidate: Blob,
    },
}

impl Event {
    pub fn blobs(&self) -> Vec<BlobDigest> {
        match self {
            Event::Prepared {
                source,
                recipe,
                candidate,
                report,
                ..
            } => {
                vec![
                    source.digest,
                    recipe.digest,
                    candidate.digest,
                    report.digest,
                ]
            }
            Event::Approved { grant, approval } => vec![grant.digest, approval.digest],
            Event::Applied { candidate } => vec![candidate.digest],
        }
    }
}

/// No historical premises yet: approval and grant envelopes are material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Premises;

impl Part for Premises {
    fn encode(&self, _: Limits) -> codec::Result<Vec<u8>> {
        Ok(Vec::new())
    }
    fn decode(bytes: &[u8], _: Limits) -> codec::Result<Self> {
        bytes
            .is_empty()
            .then_some(Premises)
            .ok_or(codec::Error::Field("premises"))
    }
}

fn blob_values(blob: &Blob) -> [C<'_>; 2] {
    [C::Bytes(blob.digest.as_digest()), C::Unsigned(blob.bytes)]
}

impl Part for Event {
    fn encode(&self, limits: Limits) -> codec::Result<Vec<u8>> {
        let encoded = match self {
            Event::Prepared {
                path,
                base,
                source,
                recipe,
                candidate,
                report,
                prepared,
            } => {
                let (s, r, c, w) = (
                    blob_values(source),
                    blob_values(recipe),
                    blob_values(candidate),
                    blob_values(report),
                );
                let fields = [
                    C::Text("prepared"),
                    C::Text(path),
                    C::Text(base),
                    C::Array(&s),
                    C::Array(&r),
                    C::Array(&c),
                    C::Array(&w),
                    C::Bytes(prepared),
                ];
                cbor::encode(&C::Array(&fields), limits)?
            }
            Event::Approved { grant, approval } => {
                let (g, a) = (blob_values(grant), blob_values(approval));
                cbor::encode(
                    &C::Array(&[C::Text("approved"), C::Array(&g), C::Array(&a)]),
                    limits,
                )?
            }
            Event::Applied { candidate } => {
                let c = blob_values(candidate);
                cbor::encode(&C::Array(&[C::Text("applied"), C::Array(&c)]), limits)?
            }
        };
        Ok(encoded.into_bytes())
    }

    fn decode(bytes: &[u8], limits: Limits) -> codec::Result<Self> {
        let mut d = Decoder::new(bytes, limits)?;
        let fields = d.array()?;
        let event = match (d.text()?, fields) {
            ("prepared", 8) => Event::Prepared {
                path: d.text()?.to_owned(),
                base: d.text()?.to_owned(),
                source: blob(&mut d)?,
                recipe: blob(&mut d)?,
                candidate: blob(&mut d)?,
                report: blob(&mut d)?,
                prepared: digest(&mut d)?,
            },
            ("approved", 3) => Event::Approved {
                grant: blob(&mut d)?,
                approval: blob(&mut d)?,
            },
            ("applied", 2) => Event::Applied {
                candidate: blob(&mut d)?,
            },
            _ => return Err(codec::Error::Field("edit event")),
        };
        d.end_array()?;
        d.finish()?;
        if event.encode(limits)? != bytes {
            return Err(codec::Error::Field("noncanonical edit event"));
        }
        Ok(event)
    }
}

fn digest(d: &mut Decoder<'_>) -> codec::Result<[u8; 32]> {
    d.bytes()?
        .try_into()
        .map_err(|_| codec::Error::Field("digest"))
}

fn blob(d: &mut Decoder<'_>) -> codec::Result<Blob> {
    if d.array()? != 2 {
        return Err(codec::Error::Field("blob"));
    }
    let digest = BlobDigest::from_digest(digest(d)?);
    let bytes = d.unsigned()?;
    d.end_array()?;
    Ok(Blob { digest, bytes })
}

/// Order and binding rules for one edit run.
pub struct Flow;

impl Validator<Event, Premises> for Flow {
    fn validate(
        &self,
        prefix: &[Record<Event, Premises>],
        record: &Record<Event, Premises>,
    ) -> bool {
        let carried = record
            .payload
            .blobs()
            .iter()
            .all(|b| record.closure.contains(b));
        let follows = match (&record.payload, prefix) {
            (Event::Prepared { path, .. }, []) => !path.is_empty(),
            (Event::Approved { .. }, [only]) => matches!(only.payload, Event::Prepared { .. }),
            (Event::Applied { candidate }, [first, second]) => {
                matches!(second.payload, Event::Approved { .. })
                    && matches!(&first.payload, Event::Prepared { candidate: prepared, .. } if prepared == candidate)
            }
            _ => false,
        };
        carried && follows
    }
}
