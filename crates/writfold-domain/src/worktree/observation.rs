use super::*;
use crate::engineering::{DispatchStampDigest, EffectIntentDigest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Confirmed,
    NoEffect,
    Unknown,
    Partial,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentObservation {
    Original([u8; 32]),
    Candidate([u8; 32]),
    Other([u8; 32]),
    Unobserved,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurabilityObservation {
    Durable,
    NotDurable,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataObservation {
    Preserved,
    Changed,
    Unobserved,
}
/// Untrusted byte descriptor, NEVER a native outcome capability or receipt authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectObservation {
    intent: EffectIntentDigest,
    stamp: DispatchStampDigest,
    prepared: PreparedEditDigest,
    outcome: Outcome,
    content: ContentObservation,
    durability: DurabilityObservation,
    metadata: MetadataObservation,
}
impl EffectObservation {
    pub fn new(
        intent: EffectIntentDigest,
        stamp: DispatchStampDigest,
        prepared: PreparedEditDigest,
        outcome: Outcome,
        content: ContentObservation,
        durability: DurabilityObservation,
        metadata: MetadataObservation,
    ) -> Result<Self> {
        match outcome {
            Outcome::Confirmed
                if !matches!(content, ContentObservation::Candidate(_))
                    || durability != DurabilityObservation::Durable
                    || metadata != MetadataObservation::Preserved =>
            {
                return Err(InvalidWorktree);
            }
            Outcome::NoEffect
                if !matches!(content, ContentObservation::Original(_))
                    || metadata != MetadataObservation::Preserved =>
            {
                return Err(InvalidWorktree);
            }
            _ => {}
        }
        Ok(Self {
            intent,
            stamp,
            prepared,
            outcome,
            content,
            durability,
            metadata,
        })
    }
    pub fn intent(&self) -> EffectIntentDigest {
        self.intent
    }
    pub fn stamp(&self) -> DispatchStampDigest {
        self.stamp
    }
    pub fn prepared(&self) -> PreparedEditDigest {
        self.prepared
    }
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn content(&self) -> &ContentObservation {
        &self.content
    }
    pub fn durability(&self) -> DurabilityObservation {
        self.durability
    }
    pub fn metadata(&self) -> MetadataObservation {
        self.metadata
    }
    pub fn observed_sha256(&self) -> Option<&[u8; 32]> {
        match &self.content {
            ContentObservation::Original(hash)
            | ContentObservation::Candidate(hash)
            | ContentObservation::Other(hash) => Some(hash),
            ContentObservation::Unobserved => None,
        }
    }
}
