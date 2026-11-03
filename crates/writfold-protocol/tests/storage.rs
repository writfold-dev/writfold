use writfold_domain::{
    LegacyDoltPin, NativeFormatVersion, OmnigraphPin, OmnigraphToolGeneration, SchemaEpoch,
    StoragePin,
};
use writfold_protocol::storage::{ProfileError, ReadPinError, ReaderProfile, admit_read};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PRODUCER: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const READER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OTHER: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const COMMIT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const NEXT_COMMIT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

struct Fields {
    kb: &'static str,
    graph: &'static str,
    commit: &'static str,
    epoch: &'static str,
    schema: &'static str,
    query: &'static str,
    profile: &'static str,
    format: u64,
    producer: &'static str,
}

impl Default for Fields {
    fn default() -> Self {
        Self {
            kb: "kb-1",
            graph: "Graph-1",
            commit: COMMIT,
            epoch: "epoch-1",
            schema: PRODUCER,
            query: READER,
            profile: "profile-1",
            format: 2,
            producer: PRODUCER,
        }
    }
}

impl Fields {
    fn pin(&self) -> Result<StoragePin, Box<dyn std::error::Error>> {
        Ok(StoragePin::Omnigraph(OmnigraphPin::new(
            self.kb.parse()?,
            self.graph.parse()?,
            self.commit.parse()?,
            SchemaEpoch::new(
                self.epoch.parse()?,
                self.schema.parse()?,
                self.query.parse()?,
            ),
            self.profile.parse()?,
            NativeFormatVersion::new(self.format)?,
            self.producer.parse()?,
        )))
    }
}

fn profile(
    id: &str,
    generation: &str,
    formats: &[u64],
) -> Result<ReaderProfile, Box<dyn std::error::Error>> {
    let formats = formats
        .iter()
        .map(|format| NativeFormatVersion::new(*format))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReaderProfile::new(
        id.parse()?,
        generation.parse()?,
        formats,
    )?)
}

fn legacy() -> Result<StoragePin, Box<dyn std::error::Error>> {
    Ok(StoragePin::LegacyDolt(LegacyDoltPin::new(
        "kb-1".parse()?,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse()?,
    )))
}

#[test]
fn profile_refuses_empty_and_nonadjacent_duplicate_formats() -> TestResult {
    assert_eq!(
        ReaderProfile::new("profile-1".parse()?, READER.parse()?, vec![]),
        Err(ProfileError::EmptyFormats)
    );
    let one = NativeFormatVersion::new(1)?;
    let three = NativeFormatVersion::new(3)?;
    for formats in [vec![one, one], vec![one, three, one]] {
        assert_eq!(
            ReaderProfile::new("profile-1".parse()?, READER.parse()?, formats),
            Err(ProfileError::DuplicateFormat)
        );
    }
    Ok(())
}

#[test]
fn historical_producer_can_match_a_distinct_reader_and_borrows_only_expected() -> TestResult {
    let expected = Fields::default().pin()?;
    let matched = {
        let observed = Fields::default().pin()?;
        let declared = profile("profile-1", READER, &[3, 2, 1])?;
        let actual_reader: OmnigraphToolGeneration = READER.parse()?;
        admit_read(&expected, &observed, &declared, &actual_reader)?
    };
    assert_eq!(matched.pin().producer().as_str(), PRODUCER);
    Ok(())
}

type Mutation = (fn(&mut Fields), ReadPinError);
const IDENTITY_MUTATIONS: [Mutation; 10] = [
    (|pin| pin.kb = "kb-2", ReadPinError::KbMismatch),
    (|pin| pin.graph = "graph-1", ReadPinError::GraphMismatch),
    (|pin| pin.commit = NEXT_COMMIT, ReadPinError::CommitMismatch),
    (|pin| pin.epoch = "epoch-2", ReadPinError::EpochMismatch),
    (|pin| pin.schema = OTHER, ReadPinError::SchemaMismatch),
    (|pin| pin.query = OTHER, ReadPinError::QueryMismatch),
    (
        |pin| pin.profile = "profile-2",
        ReadPinError::StorageProfileMismatch,
    ),
    (|pin| pin.format = 3, ReadPinError::NativeFormatMismatch),
    (
        |pin| pin.producer = OTHER,
        ReadPinError::ProducerGenerationMismatch,
    ),
    // A producer equal to the reader still cannot replace the recorded producer.
    (
        |pin| pin.producer = READER,
        ReadPinError::ProducerGenerationMismatch,
    ),
];

#[test]
fn every_observed_identity_component_is_compared_exactly_in_both_directions() -> TestResult {
    let expected = Fields::default().pin()?;
    let declared = profile("profile-1", READER, &[2, 3])?;
    let reader = READER.parse()?;
    for (mutate, error) in IDENTITY_MUTATIONS {
        let mut fields = Fields::default();
        mutate(&mut fields);
        let observed = fields.pin()?;
        assert_eq!(
            admit_read(&expected, &observed, &declared, &reader),
            Err(error)
        );
        assert_eq!(
            admit_read(&observed, &expected, &declared, &reader),
            Err(error)
        );
    }
    Ok(())
}

#[test]
fn identity_precedence_is_stable_even_with_all_later_checks_failing() -> TestResult {
    let expected = Fields::default().pin()?;
    let declared = profile("wrong-profile", OTHER, &[1])?;
    let reader = READER.parse()?;
    // Accumulate mismatches from lowest to highest precedence.
    let mut fields = Fields::default();
    for (mutate, error) in IDENTITY_MUTATIONS.into_iter().rev() {
        mutate(&mut fields);
        assert_eq!(
            admit_read(&expected, &fields.pin()?, &declared, &reader),
            Err(error)
        );
    }
    Ok(())
}

#[test]
fn hybrid_commit_never_aliases_nonce_base_or_different_slot() -> TestResult {
    let fields = Fields {
        commit: "hb1.01ARZ3NDEKTSV4RRFFQ69G5FAV.4.01ARZ3NDEKTSV4RRFFQ69G5FAW",
        ..Fields::default()
    };
    let expected = fields.pin()?;
    let declared = profile("profile-1", READER, &[2])?;
    let reader = READER.parse()?;
    for commit in [
        COMMIT,
        NEXT_COMMIT,
        "hb1.01ARZ3NDEKTSV4RRFFQ69G5FAV.5.01ARZ3NDEKTSV4RRFFQ69G5FAW",
        "hb1.01ARZ3NDEKTSV4RRFFQ69G5FAV.4.01ARZ3NDEKTSV4RRFFQ69G5FAX",
        "hb1.01ARZ3NDEKTSV4RRFFQ69G5FAX.4.01ARZ3NDEKTSV4RRFFQ69G5FAW",
    ] {
        let observed = Fields {
            commit,
            ..Fields::default()
        }
        .pin()?;
        assert_eq!(
            admit_read(&expected, &observed, &declared, &reader),
            Err(ReadPinError::CommitMismatch)
        );
        assert_eq!(
            admit_read(&observed, &expected, &declared, &reader),
            Err(ReadPinError::CommitMismatch)
        );
    }
    let observed = fields.pin()?;
    assert_eq!(
        admit_read(&expected, &observed, &declared, &reader)?
            .pin()
            .commit()
            .as_str(),
        fields.commit
    );
    Ok(())
}

#[test]
fn readable_formats_are_exact_inventory_members_not_a_range_or_floor() -> TestResult {
    let declared = profile("profile-1", READER, &[5, 1, 3])?;
    let reader = READER.parse()?;
    for format in [2, 4, 6, u64::MAX] {
        let expected = Fields {
            format,
            ..Fields::default()
        }
        .pin()?;
        assert_eq!(
            admit_read(&expected, &expected, &declared, &reader),
            Err(ReadPinError::UnsupportedNativeFormat)
        );
    }
    for format in [1, 3, 5] {
        let expected = Fields {
            format,
            ..Fields::default()
        }
        .pin()?;
        assert_eq!(
            admit_read(&expected, &expected, &declared, &reader)?
                .pin()
                .native_format()
                .get(),
            format
        );
    }
    Ok(())
}

#[test]
fn reader_profile_generation_and_format_refusals_have_fixed_precedence() -> TestResult {
    let expected = Fields::default().pin()?;
    let reader: OmnigraphToolGeneration = READER.parse()?;
    let wrong_profile = profile("wrong-profile", OTHER, &[1])?;
    assert_eq!(
        admit_read(&expected, &expected, &wrong_profile, &reader),
        Err(ReadPinError::ReaderProfileMismatch)
    );
    let wrong_generation = profile("profile-1", OTHER, &[1])?;
    assert_eq!(
        admit_read(&expected, &expected, &wrong_generation, &reader),
        Err(ReadPinError::ReaderGenerationMismatch)
    );
    let format_hole = profile("profile-1", READER, &[1, 3])?;
    assert_eq!(
        admit_read(&expected, &expected, &format_hole, &reader),
        Err(ReadPinError::UnsupportedNativeFormat)
    );
    let declared = profile("profile-1", READER, &[2])?;
    for drift in [PRODUCER, OTHER] {
        assert_eq!(
            admit_read(&expected, &expected, &declared, &drift.parse()?),
            Err(ReadPinError::ReaderGenerationMismatch)
        );
    }
    let only_wrong_profile = profile("wrong-profile", READER, &[2])?;
    assert_eq!(
        admit_read(&expected, &expected, &only_wrong_profile, &reader),
        Err(ReadPinError::ReaderProfileMismatch)
    );
    Ok(())
}

#[test]
fn legacy_is_inspectable_but_refused_before_identity_and_reader_checks() -> TestResult {
    let legacy = legacy()?;
    let StoragePin::LegacyDolt(pin) = &legacy else {
        return Err("expected legacy test fixture".into());
    };
    assert_eq!(pin.kb().as_str(), "kb-1");
    assert_eq!(pin.commit().as_str(), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let expected = Fields::default().pin()?;
    let declared = profile("wrong-profile", OTHER, &[1])?;
    let reader = READER.parse()?;
    assert_eq!(
        admit_read(&legacy, &legacy, &declared, &reader),
        Err(ReadPinError::ExpectedLegacy)
    );
    assert_eq!(
        admit_read(&legacy, &expected, &declared, &reader),
        Err(ReadPinError::ExpectedLegacy)
    );
    assert_eq!(
        admit_read(&expected, &legacy, &declared, &reader),
        Err(ReadPinError::ObservedBackendMismatch)
    );
    Ok(())
}
