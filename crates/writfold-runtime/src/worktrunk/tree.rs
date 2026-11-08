use super::{Result, WorktrunkError, profile};
use writfold_domain::GitObjectFormat;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Kind {
    Tree,
    Blob,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Blob => "blob",
        }
    }
}

pub(super) struct Entry<'a> {
    pub kind: Kind,
    pub object: &'a str,
    pub bytes: u64,
}

/// Borrow the complete NUL-delimited native result, charging every checkout
/// path even when several files share one immutable blob object.
pub(super) struct Entries<'a> {
    remaining: &'a [u8],
    format: GitObjectFormat,
    checkout: u64,
}

impl<'a> Entries<'a> {
    pub fn new(bytes: &'a [u8], format: GitObjectFormat, checkout: u64) -> Result<Self> {
        if !bytes.is_empty() && bytes.last() != Some(&0) {
            return Err(WorktrunkError::Observation("tree terminator"));
        }
        Ok(Self {
            remaining: bytes,
            format,
            checkout,
        })
    }
}

impl<'a> Iterator for Entries<'a> {
    type Item = Result<Entry<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let end = self.remaining.iter().position(|byte| *byte == 0)?;
        let (row, rest) = self.remaining.split_at(end);
        self.remaining = &rest[1..];
        Some(parse(row, self.format).and_then(|entry| {
            self.checkout = self
                .checkout
                .checked_sub(entry.bytes)
                .ok_or(WorktrunkError::Profile("checkout byte budget"))?;
            Ok(entry)
        }))
    }
}

pub(super) fn object_id(format: GitObjectFormat, value: &str) -> Result<()> {
    let length = match format {
        GitObjectFormat::Sha1 => 40,
        GitObjectFormat::Sha256 => 64,
    };
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WorktrunkError::Observation("canonical native object ID"));
    }
    Ok(())
}

fn parse(row: &[u8], format: GitObjectFormat) -> Result<Entry<'_>> {
    let tab = row
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or(WorktrunkError::Observation("tree entry"))?;
    let header =
        std::str::from_utf8(&row[..tab]).map_err(|_| WorktrunkError::Observation("tree header"))?;
    let mut fields = header.split_ascii_whitespace();
    let (mode, kind, object, size) = (fields.next(), fields.next(), fields.next(), fields.next());
    let (kind, bytes) = match (mode, kind, size) {
        (Some("040000"), Some("tree"), Some("-")) => (Kind::Tree, 0),
        (Some("100644" | "100755"), Some("blob"), Some(size))
            if !size.is_empty() && size.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            (
                Kind::Blob,
                size.parse()
                    .map_err(|_| WorktrunkError::Observation("blob size"))?,
            )
        }
        _ => return Err(WorktrunkError::Profile("regular checkout files only")),
    };
    if fields.next().is_some() {
        return Err(WorktrunkError::Observation("tree header fields"));
    }
    let object = object.ok_or(WorktrunkError::Observation("tree object ID"))?;
    object_id(format, object)?;
    let path = std::str::from_utf8(&row[tab + 1..])
        .map_err(|_| WorktrunkError::Profile("UTF8 tree path"))?;
    profile::relative(path)?;
    if path == ".config/wt.toml"
        || path
            .split('/')
            .any(|part| matches!(part, ".gitattributes" | ".gitmodules" | ".git"))
    {
        return Err(WorktrunkError::Profile("project/checkout configuration"));
    }
    Ok(Entry {
        kind,
        object,
        bytes,
    })
}
