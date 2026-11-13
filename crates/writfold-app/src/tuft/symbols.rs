//! Universal-ctags JSON is navigation evidence, never a completeness oracle.
use super::search::{Collector, Found};
use crate::trial::Result;
use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::Path;
use tuft::ir::{FindPlan, MatcherPlan, SymbolKind};
use tuft::view::{IndexCoverage, Stop};
use writfold_domain::citation::Coverage;
use writfold_runtime::fs::RelativePath;

struct Tag {
    name: String,
    path: String,
    kind: SymbolKind,
    first: usize,
    last: usize,
}

pub(super) fn find(
    index: &Path,
    plan: &FindPlan,
    files: BTreeMap<String, Vec<u8>>,
) -> Result<Found> {
    let MatcherPlan::Symbol { kind, name } = &plan.matcher else {
        return Err("index search needs a symbol matcher".into());
    };
    let (bytes, metadata) = verified(index, &plan.commit)?;
    let text = std::str::from_utf8(&bytes)?;
    let mut ranges: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        let tag = parse(line).map_err(|error| format!("ctags record {}: {error}", number + 1))?;
        let Some(tag) = tag else { continue };
        if !tuft::scope::matches(&plan.scope, &tag.path) {
            continue;
        }
        let bytes = files
            .get(&tag.path)
            .ok_or_else(|| format!("index path {} is absent at its commit", tag.path))?;
        let range = line_range(bytes, tag.first, tag.last)?;
        if tag.kind == *kind && tag.name == *name {
            ranges.entry(tag.path).or_default().push(range);
        }
    }
    let total = files.len();
    let mut collector = Collector::new(plan);
    let mut stop = None;
    for (path, mut positions) in ranges {
        positions.sort_unstable();
        positions.dedup();
        let bytes = files.get(&path).ok_or("index match without pinned bytes")?;
        if !collector.add(&path, bytes, &positions) {
            stop = Some(Stop::HitLimit);
            break;
        }
    }
    let mut found = collector.finish(
        Coverage::Measured {
            examined: total,
            total: total as u128,
        },
        stop,
    );
    found.report.index = Some(metadata);
    Ok(found)
}

fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("index or sidecar exceeds byte budget".into());
    }
    Ok(bytes)
}

#[derive(serde::Deserialize)]
struct Metadata {
    commit: String,
    sha256: String,
}

fn verified(index: &Path, commit: &str) -> Result<(Vec<u8>, IndexCoverage)> {
    let mut sidecar = index.as_os_str().to_os_string();
    sidecar.push(".meta.json");
    let meta: Metadata = serde_json::from_slice(&bounded(Path::new(&sidecar), 4096)?)?;
    if meta.commit != commit {
        return Err(
            "index commit differs from requested commit; rebuild the index at this commit".into(),
        );
    }
    let bytes = bounded(index, 64 << 20)?;
    let digest = tuft::digest_hex(&bytes);
    if meta.sha256 != digest {
        return Err(
            "index SHA-256 differs from sidecar; regenerate the index and its metadata together"
                .into(),
        );
    }
    Ok((
        bytes,
        IndexCoverage {
            commit: meta.commit,
            sha256: digest,
        },
    ))
}

#[derive(serde::Deserialize)]
#[serde(tag = "_type")]
enum Record {
    #[serde(rename = "tag")]
    Tag(Input),
    #[serde(rename = "ptag")]
    Metadata,
}

#[derive(serde::Deserialize)]
struct Input {
    name: String,
    path: String,
    line: usize,
    end: Option<usize>,
    roles: Option<Labels>,
    extras: Option<Labels>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Labels {
    Text(String),
    List(Vec<String>),
}

impl Labels {
    fn values(&self) -> impl Iterator<Item = &str> {
        let (text, list) = match self {
            Self::Text(text) => (Some(text.as_str()), None),
            Self::List(list) => (None, Some(list.as_slice())),
        };
        text.into_iter()
            .flat_map(|s| s.split(','))
            .chain(list.into_iter().flatten().map(String::as_str))
    }
}

fn parse(line: &str) -> Result<Option<Tag>> {
    let Record::Tag(mut input) = serde_json::from_str(line)? else {
        return Ok(None);
    };
    if input.name.is_empty() {
        return Err("index tag needs nonempty name".into());
    }
    if input.path.starts_with("./") {
        input.path.drain(..2);
    }
    RelativePath::new(&input.path)?;
    let first = input.line;
    let last = input.end.unwrap_or(first);
    if first == 0 || last == 0 {
        return Err("index tag needs positive line/end".into());
    }
    if last < first {
        return Err("index end precedes line".into());
    }
    let kind = roles(input.roles.as_ref())?;
    let reference = input
        .extras
        .as_ref()
        .is_some_and(|r| r.values().any(|v| v == "reference"));
    if reference && kind == SymbolKind::Def {
        return Err("reference tag needs non-definition roles".into());
    }
    Ok(Some(Tag {
        name: input.name,
        path: input.path,
        kind,
        first,
        last,
    }))
}

fn roles(value: Option<&Labels>) -> Result<SymbolKind> {
    let Some(value) = value else {
        return Ok(SymbolKind::Def);
    };
    let mut count = 0;
    let mut definition = false;
    for role in value.values() {
        if role.is_empty()
            || !role
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("empty or malformed ctags roles".into());
        }
        count += 1;
        definition |= role == "def";
    }
    if count == 0 {
        return Err("empty or malformed ctags roles".into());
    }
    if definition {
        if count != 1 {
            return Err("definition mixed with reference roles".into());
        }
        Ok(SymbolKind::Def)
    } else {
        Ok(SymbolKind::Refs)
    }
}

fn line_range(bytes: &[u8], first: usize, last: usize) -> Result<(usize, usize)> {
    let mut start = 0;
    let mut lo = None;
    let mut line = 1;
    for (offset, byte) in bytes.iter().enumerate() {
        if line == first && lo.is_none() {
            lo = Some(start);
        }
        if *byte == b'\n' || offset + 1 == bytes.len() {
            if line == last {
                return Ok((lo.ok_or("index line is outside pinned blob")?, offset + 1));
            }
            line += 1;
            start = offset + 1;
        }
    }
    Err("index line/end is outside pinned blob".into())
}
