use writfold_domain::{
    GraphId, KbId, NativeFormatVersion, OmnigraphToolGeneration, ReleaseId, SchemaEpoch,
    StoragePin, StorageProfileId,
};

/// Destination identity without a not-yet-computed native commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageTarget {
    kb: KbId,
    graph: GraphId,
    epoch: SchemaEpoch,
    profile: StorageProfileId,
    native_format: NativeFormatVersion,
    producer: OmnigraphToolGeneration,
}
impl StorageTarget {
    pub fn new(
        kb: KbId,
        graph: GraphId,
        epoch: SchemaEpoch,
        profile: StorageProfileId,
        native_format: NativeFormatVersion,
        producer: OmnigraphToolGeneration,
    ) -> Self {
        Self {
            kb,
            graph,
            epoch,
            profile,
            native_format,
            producer,
        }
    }
    pub fn kb(&self) -> &KbId {
        &self.kb
    }
    pub fn graph(&self) -> &GraphId {
        &self.graph
    }
    pub fn epoch(&self) -> &SchemaEpoch {
        &self.epoch
    }
    pub fn profile(&self) -> &StorageProfileId {
        &self.profile
    }
    pub fn native_format(&self) -> &NativeFormatVersion {
        &self.native_format
    }
    pub fn producer(&self) -> &OmnigraphToolGeneration {
        &self.producer
    }
}

/// Exact completed release and its retained storage pin, supplied by the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageBase {
    release: ReleaseId,
    pin: StoragePin,
}
impl StorageBase {
    pub fn new(release: ReleaseId, pin: StoragePin) -> Self {
        Self { release, pin }
    }
    pub fn release(&self) -> &ReleaseId {
        &self.release
    }
    pub fn pin(&self) -> &StoragePin {
        &self.pin
    }
}
