//! Edit-flow ledger records for `writfold-trial-worktree/1`, and their pure
//! validator. One run is one stream: `Prepared`, `Approved`, then a durable
//! `Claimed` before any file is touched, then exactly one outcome: `Applied`
//! (installed, or reconciled as installed after a lost outcome) or `Abandoned`
//! (observed not installed). Each record names its retained material by digest
//! and length and must carry it in its closure. The validator checks order and
//! bindings only; signatures, grants and the file guard are checked by `apply`
//! against the retained bytes before `Claimed` is appended.
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
    Claimed {
        candidate: Blob,
    },
    Applied {
        candidate: Blob,
        reconciled: bool,
    },
    Abandoned {
        reason: String,
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
            Event::Claimed { candidate } | Event::Applied { candidate, .. } => {
                vec![candidate.digest]
            }
            Event::Abandoned { .. } => Vec::new(),
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
            Event::Claimed { candidate } => {
                let c = blob_values(candidate);
                cbor::encode(&C::Array(&[C::Text("claimed"), C::Array(&c)]), limits)?
            }
            Event::Applied {
                candidate,
                reconciled,
            } => {
                let c = blob_values(candidate);
                let fields = [C::Text("applied"), C::Array(&c), C::Bool(*reconciled)];
                cbor::encode(&C::Array(&fields), limits)?
            }
            Event::Abandoned { reason } => {
                cbor::encode(&C::Array(&[C::Text("abandoned"), C::Text(reason)]), limits)?
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
            ("claimed", 2) => Event::Claimed {
                candidate: blob(&mut d)?,
            },
            ("applied", 3) => Event::Applied {
                candidate: blob(&mut d)?,
                reconciled: d.boolean()?,
            },
            ("abandoned", 2) => Event::Abandoned {
                reason: d.text()?.to_owned(),
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
            (Event::Claimed { candidate }, [first, second]) => {
                matches!(second.payload, Event::Approved { .. }) && prepared(first, candidate)
            }
            (Event::Applied { candidate, .. }, [first, _, third]) => {
                matches!(third.payload, Event::Claimed { .. }) && prepared(first, candidate)
            }
            (Event::Abandoned { reason }, [_, _, third]) => {
                matches!(third.payload, Event::Claimed { .. }) && !reason.is_empty()
            }
            _ => false,
        };
        carried && follows
    }
}

/// Whether `first` prepared exactly this candidate.
fn prepared(first: &Record<Event, Premises>, candidate: &Blob) -> bool {
    matches!(&first.payload, Event::Prepared { candidate: c, .. } if c == candidate)
}
