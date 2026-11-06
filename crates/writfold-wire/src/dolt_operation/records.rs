use writfold_domain::{
    DoltDatabaseId, DoltToolGeneration, KbId, RealmId, ReleaseId, SchemaEpoch, StorageProfileId,
    dolt::DoltReadPin,
};

/// Destination identity without a not-yet-computed native commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltTarget {
    realm: RealmId,
    kb: KbId,
    database: DoltDatabaseId,
    epoch: SchemaEpoch,
    profile: StorageProfileId,
    producer: DoltToolGeneration,
}
impl DoltTarget {
    pub fn new(
        realm: RealmId,
        kb: KbId,
        database: DoltDatabaseId,
        epoch: SchemaEpoch,
        profile: StorageProfileId,
        producer: DoltToolGeneration,
    ) -> Self {
        Self {
            realm,
            kb,
            database,
            epoch,
            profile,
            producer,
        }
    }
    pub fn realm(&self) -> &RealmId {
        &self.realm
    }
    pub fn kb(&self) -> &KbId {
        &self.kb
    }
    pub fn database(&self) -> &DoltDatabaseId {
        &self.database
    }
    pub fn epoch(&self) -> &SchemaEpoch {
        &self.epoch
    }
    pub fn profile(&self) -> &StorageProfileId {
        &self.profile
    }
    pub fn producer(&self) -> &DoltToolGeneration {
        &self.producer
    }
}

/// Exact completed release and its integration pin, supplied by the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltBase {
    release: ReleaseId,
    pin: DoltReadPin,
}
impl DoltBase {
    pub fn new(release: ReleaseId, pin: DoltReadPin) -> Self {
        Self { release, pin }
    }
    pub fn release(&self) -> &ReleaseId {
        &self.release
    }
    pub fn pin(&self) -> &DoltReadPin {
        &self.pin
    }
}
