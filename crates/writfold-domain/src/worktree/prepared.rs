use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    profile: String,
    sha256: [u8; 32],
}
impl Artifact {
    pub fn new(profile: String, sha256: [u8; 32]) -> Result<Self> {
        label(&profile)?;
        Ok(Self { profile, sha256 })
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}
/// Half-open origin byte span in the retained recipe, preserved per lowered edit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecipeSpan {
    start: u64,
    end: u64,
}
impl RecipeSpan {
    pub fn new(start: u64, end: u64) -> Result<Self> {
        if start > end {
            return Err(InvalidWorktree);
        }
        Ok(Self { start, end })
    }
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn end(&self) -> u64 {
        self.end
    }
}
/// Half-open byte offsets in the original capture, never character offsets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Splice {
    start: u64,
    end: u64,
    replacement: Vec<u8>,
    origin: RecipeSpan,
}
impl Splice {
    pub fn new(start: u64, end: u64, replacement: Vec<u8>, origin: RecipeSpan) -> Result<Self> {
        if start > end {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            start,
            end,
            replacement,
            origin,
        })
    }
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn end(&self) -> u64 {
        self.end
    }
    pub fn replacement(&self) -> &[u8] {
        &self.replacement
    }
    pub fn origin(&self) -> RecipeSpan {
        self.origin
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadRange {
    start: u64,
    bytes: Vec<u8>,
}
impl ReadRange {
    pub fn new(start: u64, bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() || start.checked_add(bytes.len() as u64).is_none() {
            return Err(InvalidWorktree);
        }
        Ok(Self { start, bytes })
    }
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
/// Structurally checked proposal. Exact references/derivation require wire::validate_prepared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedEdit {
    recipe: RecipeDigest,
    capture: CaptureDigest,
    candidate: CandidateDigest,
    compiler: Artifact,
    kernel: Artifact,
    splices: Vec<Splice>,
    read_set: Vec<ReadRange>,
}
impl PreparedEdit {
    pub fn new(
        recipe: RecipeDigest,
        capture: CaptureDigest,
        candidate: CandidateDigest,
        compiler: Artifact,
        kernel: Artifact,
        splices: Vec<Splice>,
        read_set: Vec<ReadRange>,
    ) -> Result<Self> {
        if splices.windows(2).any(|w| {
            w[0].end > w[1].start
                || (w[0].end == w[1].start && (w[0].start == w[0].end || w[1].start == w[1].end))
        }) || read_set
            .windows(2)
            .any(|w| w[0].start + w[0].bytes.len() as u64 > w[1].start)
        {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            recipe,
            capture,
            candidate,
            compiler,
            kernel,
            splices,
            read_set,
        })
    }
    pub fn recipe(&self) -> RecipeDigest {
        self.recipe
    }
    pub fn capture(&self) -> CaptureDigest {
        self.capture
    }
    pub fn candidate(&self) -> CandidateDigest {
        self.candidate
    }
    pub fn compiler(&self) -> &Artifact {
        &self.compiler
    }
    pub fn kernel(&self) -> &Artifact {
        &self.kernel
    }
    pub fn splices(&self) -> &[Splice] {
        &self.splices
    }
    pub fn read_set(&self) -> &[ReadRange] {
        &self.read_set
    }
    pub fn validate_origins(&self, recipe: &[u8]) -> Result<()> {
        if self
            .splices
            .iter()
            .any(|s| s.origin.end > recipe.len() as u64)
        {
            return Err(InvalidWorktree);
        }
        Ok(())
    }
    /// Compare streamed derivation without allocating a second candidate buffer.
    pub fn validate_bytes(&self, original: &[u8], candidate: &[u8]) -> Result<()> {
        for read in &self.read_set {
            let start = usize::try_from(read.start).map_err(|_| InvalidWorktree)?;
            let end = start.checked_add(read.bytes.len()).ok_or(InvalidWorktree)?;
            if original.get(start..end) != Some(read.bytes.as_slice()) {
                return Err(InvalidWorktree);
            }
        }
        let mut original_at = 0;
        let mut candidate_at: usize = 0;
        for splice in &self.splices {
            let start = usize::try_from(splice.start).map_err(|_| InvalidWorktree)?;
            let end = usize::try_from(splice.end).map_err(|_| InvalidWorktree)?;
            let unchanged = original.get(original_at..start).ok_or(InvalidWorktree)?;
            if end > original.len() {
                return Err(InvalidWorktree);
            }
            for part in [unchanged, splice.replacement.as_slice()] {
                let next = candidate_at
                    .checked_add(part.len())
                    .ok_or(InvalidWorktree)?;
                if candidate.get(candidate_at..next) != Some(part) {
                    return Err(InvalidWorktree);
                }
                candidate_at = next;
            }
            original_at = end;
        }
        if candidate.get(candidate_at..) != original.get(original_at..) {
            return Err(InvalidWorktree);
        }
        Ok(())
    }
}
