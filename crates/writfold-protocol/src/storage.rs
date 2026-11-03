//! Exact, allocation-free storage-pin and reader compatibility decisions.
//!
//! All inputs are caller-supplied declarations or observations. A match establishes
//! typed consistency only, not authentication, tool admission, retained storage,
//! durable state, a reader lease, or permission to perform IO. Adapters must supply
//! those authorities separately. This module has no storage IO or writer path.
use std::{error::Error, fmt};
use vstd::prelude::*;
use writfold_domain::{
    NativeFormatVersion, OmnigraphPin, OmnigraphToolGeneration, StoragePin, StorageProfileId,
};

verus! {

/// A typed readable-format inventory, not evidence admitting an actual runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderProfile {
    id: StorageProfileId,
    generation: OmnigraphToolGeneration,
    native_formats: Vec<NativeFormatVersion>,
}

#[cfg(verus_keep_ghost)]
pub ghost struct ReaderProfileView {
    pub id: Seq<char>,
    pub generation: Seq<char>,
    pub native_formats: Seq<NativeFormatVersion>,
}

#[cfg(verus_keep_ghost)]
impl View for ReaderProfile {
    type V = ReaderProfileView;
    closed spec fn view(&self) -> ReaderProfileView {
        ReaderProfileView {
            id: self.id@,
            generation: self.generation@,
            native_formats: self.native_formats@,
        }
    }
}

pub open spec fn unique_formats(formats: Seq<NativeFormatVersion>) -> bool {
    forall|i: int, j: int| 0 <= i < j < formats.len()
        ==> #[trigger] formats[i] != #[trigger] formats[j]
}

impl ReaderProfile {
    pub open spec fn valid(&self) -> bool {
        self@.native_formats.len() > 0 && unique_formats(self@.native_formats)
    }

    /// Takes ownership without copying or reordering; specifies both refusals
    /// and exact successful ownership of the caller's complete inventory.
    pub fn new(
        id: StorageProfileId,
        generation: OmnigraphToolGeneration,
        native_formats: Vec<NativeFormatVersion>,
    ) -> (result: Result<Self, ProfileError>)
        ensures
            result.is_ok() <==> native_formats.len() > 0 && unique_formats(native_formats@),
            result.is_ok() ==> result.unwrap().valid(),
            result.is_ok() ==> result.unwrap()@ == (ReaderProfileView {
                id: id@, generation: generation@, native_formats: native_formats@,
            }),
            result == Err(ProfileError::EmptyFormats) <==> native_formats.len() == 0,
            result == Err(ProfileError::DuplicateFormat) <==>
                native_formats.len() > 0 && !unique_formats(native_formats@),
    {
        if native_formats.is_empty() {
            return Err(ProfileError::EmptyFormats);
        }
        if !formats_are_unique(&native_formats) {
            return Err(ProfileError::DuplicateFormat);
        }
        Ok(Self { id, generation, native_formats })
    }

    pub fn id(&self) -> (result: &StorageProfileId)
        ensures result@ == self@.id,
    {
        &self.id
    }

    pub fn generation(&self) -> (result: &OmnigraphToolGeneration)
        ensures result@ == self@.generation,
    {
        &self.generation
    }

    pub fn native_formats(&self) -> (result: &[NativeFormatVersion])
        ensures result@ == self@.native_formats,
    {
        &self.native_formats
    }
}

fn formats_are_unique(formats: &[NativeFormatVersion]) -> (unique: bool)
    ensures unique <==> unique_formats(formats@),
{
    let mut index = 0usize;
    while index < formats.len()
        invariant
            index <= formats.len(),
            forall|i: int, j: int| 0 <= i < j < index
                ==> #[trigger] formats@[i] != #[trigger] formats@[j],
        decreases formats.len() - index,
    {
        let mut prior = 0usize;
        while prior < index
            invariant
                prior <= index < formats.len(),
                forall|i: int, j: int| 0 <= i < j < index
                    ==> #[trigger] formats@[i] != #[trigger] formats@[j],
                forall|i: int| 0 <= i < prior ==> formats@[i] != formats@[index as int],
            decreases index - prior,
        {
            if formats[prior] == formats[index] {
                return false;
            }
            prior += 1;
        }
        index += 1;
    }
    true
}

fn contains_format(formats: &[NativeFormatVersion], wanted: &NativeFormatVersion) -> (found: bool)
    ensures found <==> formats@.contains(*wanted),
{
    let mut index = 0usize;
    while index < formats.len()
        invariant
            index <= formats.len(),
            forall|i: int| 0 <= i < index ==> formats@[i] != *wanted,
        decreases formats.len() - index,
    {
        if formats[index] == *wanted {
            return true;
        }
        index += 1;
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileError {
    EmptyFormats,
    DuplicateFormat,
}

} // verus!

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyFormats => "reader profile has no native formats",
            Self::DuplicateFormat => "reader profile repeats a native format",
        })
    }
}

impl Error for ProfileError {}

verus! {

/// Borrows the expected pin after a successful pure consistency decision.
/// Not an authenticated observation, read capability, or production proof receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchedStoragePin<'a> {
    pin: &'a OmnigraphPin,
}

#[cfg(verus_keep_ghost)]
impl<'a> View for MatchedStoragePin<'a> {
    type V = writfold_domain::OmnigraphPinView;
    closed spec fn view(&self) -> Self::V { self.pin@ }
}

impl<'a> MatchedStoragePin<'a> {
    pub fn pin(&self) -> (result: &'a OmnigraphPin)
        ensures result@ == self@,
    {
        self.pin
    }
}

/// Refusals are ordered by the checks documented on [`admit_read`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadPinError {
    ExpectedLegacy,
    ObservedBackendMismatch,
    KbMismatch,
    GraphMismatch,
    CommitMismatch,
    EpochMismatch,
    SchemaMismatch,
    QueryMismatch,
    StorageProfileMismatch,
    NativeFormatMismatch,
    ProducerGenerationMismatch,
    ReaderProfileMismatch,
    ReaderGenerationMismatch,
    UnsupportedNativeFormat,
}

} // verus!

impl fmt::Display for ReadPinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ExpectedLegacy => "legacy pins cannot use the Omnigraph read path",
            Self::ObservedBackendMismatch => "observed storage backend differs",
            Self::KbMismatch => "observed knowledge base differs",
            Self::GraphMismatch => "observed graph differs",
            Self::CommitMismatch => "observed exact graph commit differs",
            Self::EpochMismatch => "observed schema epoch differs",
            Self::SchemaMismatch => "observed schema digest differs",
            Self::QueryMismatch => "observed query contract digest differs",
            Self::StorageProfileMismatch => "observed storage profile differs",
            Self::NativeFormatMismatch => "observed native format differs",
            Self::ProducerGenerationMismatch => "observed producer generation differs",
            Self::ReaderProfileMismatch => "reader profile differs from the storage pin",
            Self::ReaderGenerationMismatch => "actual reader differs from the profile generation",
            Self::UnsupportedNativeFormat => "native format is absent from the reader inventory",
        })
    }
}

impl Error for ReadPinError {}

verus! {

/// Declarative identity and compatibility obligation; reader and producer
/// generations deliberately occupy different roles.
pub open spec fn read_consistent(
    expected: &StoragePin, observed: &StoragePin,
    profile: &ReaderProfile, actual_reader: &OmnigraphToolGeneration,
) -> bool {
    match (expected, observed) {
        (StoragePin::Omnigraph(pin), StoragePin::Omnigraph(observed)) =>
            pin@ == observed@
            && pin@.profile == profile@.id
            && actual_reader@ == profile@.generation
            && profile@.native_formats.contains(pin@.native_format),
        _ => false,
    }
}

/// Ordered refusal specification, separate from the conjunctive success
/// obligation. No external authority or storage correspondence is inferred.
pub open spec fn read_refusal(
    expected: &StoragePin, observed: &StoragePin,
    profile: &ReaderProfile, actual_reader: &OmnigraphToolGeneration,
) -> Option<ReadPinError> {
    match expected {
        StoragePin::LegacyDolt(_) => Some(ReadPinError::ExpectedLegacy),
        StoragePin::Omnigraph(pin) => match observed {
            StoragePin::LegacyDolt(_) => Some(ReadPinError::ObservedBackendMismatch),
            StoragePin::Omnigraph(observed) => {
                if pin@.kb != observed@.kb { Some(ReadPinError::KbMismatch) }
                else if pin@.graph != observed@.graph { Some(ReadPinError::GraphMismatch) }
                else if pin@.commit != observed@.commit { Some(ReadPinError::CommitMismatch) }
                else if pin@.epoch.id != observed@.epoch.id { Some(ReadPinError::EpochMismatch) }
                else if pin@.epoch.schema != observed@.epoch.schema { Some(ReadPinError::SchemaMismatch) }
                else if pin@.epoch.query != observed@.epoch.query { Some(ReadPinError::QueryMismatch) }
                else if pin@.profile != observed@.profile { Some(ReadPinError::StorageProfileMismatch) }
                else if pin@.native_format != observed@.native_format { Some(ReadPinError::NativeFormatMismatch) }
                else if pin@.producer != observed@.producer { Some(ReadPinError::ProducerGenerationMismatch) }
                else if pin@.profile != profile@.id { Some(ReadPinError::ReaderProfileMismatch) }
                else if actual_reader@ != profile@.generation { Some(ReadPinError::ReaderGenerationMismatch) }
                else if !profile@.native_formats.contains(pin@.native_format) {
                    Some(ReadPinError::UnsupportedNativeFormat)
                } else { None }
            },
        },
    }
}

/// Compare exact pin identities, then check the declared reader compatibility.
///
/// Deterministic refusal precedence: expected legacy; observed backend; KB;
/// graph; exact commit (no nonce aliases); epoch ID; schema digest; query digest;
/// storage profile ID; native format; producer generation; reader profile ID;
/// actual reader generation; exact native-format membership (not a range).
///
/// Reader generation need not equal producer generation: an explicitly compatible
/// newer reader may read historical data. No IO, allocation, cloning, authentication
/// or external admission occurs. The returned reference borrows only `expected`.
/// The contracts state complete typed consistency, exact refusal precedence and
/// return of the expected identity. They do not establish migration correctness,
/// engine refinement or exercised V15 qualification.
pub fn admit_read<'a>(
    expected: &'a StoragePin,
    observed: &StoragePin,
    profile: &ReaderProfile,
    actual_reader: &OmnigraphToolGeneration,
) -> (result: Result<MatchedStoragePin<'a>, ReadPinError>)
    ensures
        result.is_ok() <==> read_consistent(expected, observed, profile, actual_reader),
        match result {
            Ok(matched) => match expected {
                StoragePin::Omnigraph(pin) => matched@ == pin@,
                _ => false,
            },
            Err(error) => read_refusal(expected, observed, profile, actual_reader) == Some(error),
        },
{
    let StoragePin::Omnigraph(expected) = expected else {
        return Err(ReadPinError::ExpectedLegacy);
    };
    let StoragePin::Omnigraph(observed) = observed else {
        return Err(ReadPinError::ObservedBackendMismatch);
    };
    if expected.kb() != observed.kb() {
        return Err(ReadPinError::KbMismatch);
    }
    if expected.graph() != observed.graph() {
        return Err(ReadPinError::GraphMismatch);
    }
    if expected.commit() != observed.commit() {
        return Err(ReadPinError::CommitMismatch);
    }
    if expected.epoch().id() != observed.epoch().id() {
        return Err(ReadPinError::EpochMismatch);
    }
    if expected.epoch().schema() != observed.epoch().schema() {
        return Err(ReadPinError::SchemaMismatch);
    }
    if expected.epoch().query() != observed.epoch().query() {
        return Err(ReadPinError::QueryMismatch);
    }
    if expected.profile() != observed.profile() {
        return Err(ReadPinError::StorageProfileMismatch);
    }
    if expected.native_format() != observed.native_format() {
        return Err(ReadPinError::NativeFormatMismatch);
    }
    if expected.producer() != observed.producer() {
        return Err(ReadPinError::ProducerGenerationMismatch);
    }
    if expected.profile() != profile.id() {
        return Err(ReadPinError::ReaderProfileMismatch);
    }
    if actual_reader != profile.generation() {
        return Err(ReadPinError::ReaderGenerationMismatch);
    }
    if !contains_format(profile.native_formats(), expected.native_format()) {
        return Err(ReadPinError::UnsupportedNativeFormat);
    }
    Ok(MatchedStoragePin { pin: expected })
}

} // verus!
