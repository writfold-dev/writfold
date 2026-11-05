use writfold_domain::{citation::*, *};

pub(super) type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

pub(super) struct Fixture {
    pub reference: RepresentationRef,
    pub id: CitationId,
    pub release: SnapshotId,
    pub context: ContextId,
    pub role: EvidenceRoleId,
    pub principal: ActorId,
    pub instance: InstanceId,
    pub incarnation: SessionIncarnation,
}
impl Fixture {
    pub fn new() -> TestResult<Self> {
        Ok(Self {
            reference: RepresentationRef::new(
                "realm".parse()?,
                "source1".parse()?,
                "r1".parse()?,
                "file.txt".into(),
                "bytes1".parse()?,
                RepresentationProfile::GitBlob,
            ),
            id: "citation".parse()?,
            release: "snapshot".parse()?,
            context: "context".parse()?,
            role: "evidence".parse()?,
            principal: "principal".parse()?,
            instance: "instance".parse()?,
            incarnation: "incarnation".parse()?,
        })
    }
    pub fn binding<'a>(
        &'a self,
        first: u64,
        last: u64,
        expected: &'a [u8],
    ) -> TestResult<Binding<'a>> {
        self.binding_at(&self.reference, first, last, expected)
    }
    pub fn binding_at<'a>(
        &'a self,
        reference: &'a RepresentationRef,
        first: u64,
        last: u64,
        expected: &'a [u8],
    ) -> TestResult<Binding<'a>> {
        Ok(Binding::new(
            &self.id,
            &self.release,
            &self.context,
            &self.role,
            reference,
            selector(first, last)?,
            expected,
        ))
    }
    pub fn scope(&self, table_generation: u64) -> AliasScope<'_> {
        AliasScope::new(
            self.reference.realm(),
            &self.principal,
            &self.instance,
            &self.incarnation,
            &self.release,
            &self.context,
            CitationTableGeneration::from_u64(table_generation),
        )
    }
}

pub(super) fn selector(first: u64, last: u64) -> TestResult<Selector> {
    Ok(Selector::new(
        ByteOffset::new(first)?,
        ByteOffset::new(last)?,
        CoordinateUnit::Byte,
    )
    .map_err(|error| format!("{error:?}"))?)
}
pub(super) fn allowed(binding: Binding<'_>) -> AccessDecision<'_> {
    AccessDecision::new(binding, true)
}
pub(super) fn seconds(value: u64) -> TestResult<EpochSeconds> {
    Ok(EpochSeconds::new(value)?)
}
